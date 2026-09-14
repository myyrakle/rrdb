//! Contains the [Connection] struct, which represents an individual Postgres session, and related types.

use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;

use crate::engine::ast::dml::insert::InsertQuery;
use crate::engine::ast::dml::parts::insert_values::InsertValue;
use crate::engine::ast::types::{DataType, SQLExpression, TableName};
use crate::engine::ast::{DDLStatement, DMLStatement, OtherStatement, SQLStatement};
use crate::engine::lexer::predule::Tokenizer;
use crate::engine::parser::context::ParserContext;
use crate::engine::parser::predule::Parser;
use crate::engine::server::shared_state::SharedState;
use crate::engine::types::{ExecuteColumn, ExecuteResult};
use crate::pgwire::connection::{BoundPortal, ConnectionError, ConnectionState, PreparedStatement};
use crate::pgwire::engine::{Engine, Portal, RRDBEngine};
use crate::pgwire::protocol::backend::{
    AuthenticationOk, BindComplete, CloseComplete, CommandComplete, EmptyQueryResponse,
    ErrorResponse, NoData, ParameterDescription, ParameterStatus, ParseComplete, ReadyForQuery,
    RowDescription,
};
use crate::pgwire::protocol::client::{BindFormat, ClientMessage, Close, Describe};
use crate::pgwire::protocol::{ConnectionCodec, DataRowBatch, FormatCode, Severity, SqlState};

/// Describes a connection using a specific engine.
/// Contains connection state including prepared statements and portals.
pub struct Connection {
    engine: RRDBEngine,
    state: ConnectionState,
    statements: HashMap<String, PreparedStatement>,
    portals: HashMap<String, Option<BoundPortal<RRDBEngine>>>,
    /// True while recovering from an extended-protocol error: every client
    /// message except Sync/Terminate is discarded until Sync arrives.
    skip_until_sync: bool,
    /// Whether the message currently being processed came from simple query
    /// mode (`Q`). Simple-query errors reply with ReadyForQuery immediately;
    /// extended-protocol errors instead switch into `skip_until_sync`.
    current_message_is_simple: bool,
}

impl Connection {
    /// Create a new connection from an engine instance.
    pub fn new(shared_state: SharedState) -> Self {
        Self {
            state: ConnectionState::Startup,
            statements: HashMap::new(),
            portals: HashMap::new(),
            engine: RRDBEngine { shared_state },
            skip_until_sync: false,
            current_message_is_simple: true,
        }
    }

    fn prepared_statement(&self, name: &str) -> Result<&PreparedStatement, ConnectionError> {
        Ok(self.statements.get(name).ok_or_else(|| {
            ErrorResponse::error(SqlState::INVALID_SQL_STATEMENT_NAME, "missing statement")
        })?)
    }

    fn portal(&self, name: &str) -> Result<&Option<BoundPortal<RRDBEngine>>, ConnectionError> {
        Ok(self
            .portals
            .get(name)
            .ok_or_else(|| ErrorResponse::error(SqlState::INVALID_CURSOR_NAME, "missing portal"))?)
    }

    fn portal_mut(
        &mut self,
        name: &str,
    ) -> Result<&mut Option<BoundPortal<RRDBEngine>>, ConnectionError> {
        Ok(self
            .portals
            .get_mut(name)
            .ok_or_else(|| ErrorResponse::error(SqlState::INVALID_CURSOR_NAME, "missing portal"))?)
    }

    fn parse_statement(&mut self, text: &str) -> Result<Option<SQLStatement>, ErrorResponse> {
        let tokens = match Tokenizer::string_to_tokens(text.into()) {
            Ok(tokens) => tokens,
            Err(e) => {
                return Err(ErrorResponse::error(SqlState::SYNTAX_ERROR, e.to_string()));
            }
        };

        let mut parser = Parser::new(tokens);

        let statements = parser
            .parse(
                ParserContext::default()
                    .set_default_database(self.engine.shared_state.client_info.database.clone()),
            )
            .map_err(|e| ErrorResponse::error(SqlState::SYNTAX_ERROR, e.to_string()))?;

        match statements.len() {
            0 => Ok(None),
            1 => Ok(Some(statements[0].clone())),
            _ => Err(ErrorResponse::error(
                SqlState::SYNTAX_ERROR,
                "expected zero or one statements",
            )),
        }
    }

    fn query_has_parameters(text: &str) -> bool {
        // A scanner failure means the query is malformed; report "no parameters" and let
        // the downstream SQL parser produce the actual syntax error.
        matches!(
            scan_sql_segments(text),
            Ok(segments) if segments.iter().any(|s| matches!(s, SqlSegment::Placeholder(_))),
        )
    }

    fn quote_parameter(parameter: &Option<String>) -> String {
        match parameter {
            Some(value) => format!("'{}'", value.replace('\'', "''")),
            None => "NULL".to_string(),
        }
    }

    fn bind_query_parameters(
        query: &str,
        parameters: &[Option<String>],
    ) -> Result<String, ErrorResponse> {
        let segments = scan_sql_segments(query)?;
        let mut bound = String::with_capacity(query.len());

        for segment in segments {
            match segment {
                SqlSegment::Passthrough(text) => bound.push_str(text),
                SqlSegment::Placeholder(index) => {
                    if index == 0 || index > parameters.len() {
                        return Err(ErrorResponse::error(
                            SqlState::SYNTAX_ERROR,
                            format!("missing bind parameter ${index}"),
                        ));
                    }
                    bound.push_str(&Self::quote_parameter(&parameters[index - 1]));
                }
            }
        }

        Ok(bound)
    }

    async fn parse_parameterized_insert(
        &self,
        query: &str,
        parameters: &[Option<String>],
    ) -> Option<SQLStatement> {
        let lower = query.to_ascii_lowercase();
        let after_insert = lower.strip_prefix("insert into ")?;
        let table_start = query.len() - after_insert.len();
        let columns_start = query[table_start..].find('(')? + table_start;
        let table = query[table_start..columns_start].trim();
        let columns_end = query[columns_start + 1..].find(')')? + columns_start + 1;
        let columns = query[columns_start + 1..columns_end]
            .split(',')
            .map(str::trim)
            .filter(|column| !column.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();

        if columns.is_empty() {
            return None;
        }

        let rest = query[columns_end + 1..].trim_start();
        let rest_lower = rest.to_ascii_lowercase();
        let values_rest = rest_lower.strip_prefix("values")?;
        let values_start = rest.len() - values_rest.len();
        let values = rest[values_start..].trim_start();
        let value_start = values.find('(')?;
        let value_end = values[value_start + 1..].find(')')? + value_start + 1;
        let placeholders = values[value_start + 1..value_end]
            .split(',')
            .map(str::trim)
            .collect::<Vec<_>>();

        if placeholders.len() != columns.len() {
            return None;
        }

        let trailing = values[value_end + 1..].trim_start();
        if !trailing.is_empty() && trailing != ";" {
            return None;
        }

        let table_name = match table.split_once('.') {
            Some((database_name, table_name)) => TableName::new(
                Some(database_name.trim().to_string()),
                table_name.trim().to_string(),
            ),
            None => TableName::new(
                Some(self.engine.shared_state.client_info.database.clone()),
                table.to_string(),
            ),
        };
        let table_config = self
            .engine
            .shared_state
            .engine
            .get_table_config_cached(table_name.clone())
            .await
            .ok()?;
        let columns_map = table_config.get_columns_map();

        let mut expressions = Vec::with_capacity(placeholders.len());
        for (column_name, placeholder) in columns.iter().zip(placeholders) {
            let parameter_index = placeholder.strip_prefix('$')?.parse::<usize>().ok()?;
            if parameter_index == 0 || parameter_index > parameters.len() {
                return None;
            }

            let column = columns_map.get(column_name)?;
            let expression = Self::parameter_expression_for_column(
                &parameters[parameter_index - 1],
                &column.data_type,
            )?;
            expressions.push(Some(expression));
        }

        Some(
            InsertQuery::builder()
                .set_into_table(table_name)
                .set_columns(columns)
                .set_values(vec![InsertValue { list: expressions }])
                .build()
                .into(),
        )
    }

    fn parameter_expression_for_column(
        parameter: &Option<String>,
        data_type: &DataType,
    ) -> Option<SQLExpression> {
        let Some(value) = parameter else {
            return Some(SQLExpression::Null);
        };

        match data_type {
            DataType::Int => value.parse::<i64>().ok().map(SQLExpression::Integer),
            DataType::Float => value.parse::<f64>().ok().map(SQLExpression::Float),
            DataType::Boolean => match value.to_ascii_lowercase().as_str() {
                "true" | "t" | "1" => Some(SQLExpression::Boolean(true)),
                "false" | "f" | "0" => Some(SQLExpression::Boolean(false)),
                _ => None,
            },
            DataType::Varchar(_) => Some(SQLExpression::String(value.clone())),
        }
    }

    fn returns_rows(statement: &SQLStatement) -> bool {
        matches!(
            statement,
            SQLStatement::DML(DMLStatement::SelectQuery(_))
                | SQLStatement::Other(OtherStatement::ShowDatabases(_))
                | SQLStatement::Other(OtherStatement::ShowTables(_))
                | SQLStatement::Other(OtherStatement::DescTable(_))
                | SQLStatement::Other(OtherStatement::UseDatabase(_))
        )
    }

    fn command_tag(statement: &SQLStatement, result: &ExecuteResult) -> String {
        let num_rows = result.affected_rows.unwrap_or(result.rows.len());

        match statement {
            SQLStatement::DML(DMLStatement::SelectQuery(_)) => format!("SELECT {}", num_rows),
            SQLStatement::DML(DMLStatement::InsertQuery(_)) => format!("INSERT 0 {}", num_rows),
            SQLStatement::DML(DMLStatement::UpdateQuery(_)) => format!("UPDATE {}", num_rows),
            SQLStatement::DML(DMLStatement::DeleteQuery(_)) => format!("DELETE {}", num_rows),
            SQLStatement::DDL(DDLStatement::CreateDatabaseQuery(_)) => {
                "CREATE DATABASE".to_string()
            }
            SQLStatement::DDL(DDLStatement::CreateTableQuery(_)) => "CREATE TABLE".to_string(),
            SQLStatement::DDL(DDLStatement::DropDatabaseQuery(_)) => "DROP DATABASE".to_string(),
            SQLStatement::DDL(DDLStatement::DropTableQuery(_)) => "DROP TABLE".to_string(),
            SQLStatement::DDL(DDLStatement::AlterDatabase(_)) => "ALTER DATABASE".to_string(),
            SQLStatement::DDL(DDLStatement::AlterTableQuery(_)) => "ALTER TABLE".to_string(),
            SQLStatement::DDL(DDLStatement::CreateIndexQuery(_)) => "CREATE INDEX".to_string(),
            _ => format!("SELECT {}", num_rows),
        }
    }

    fn row_description_fields(
        columns: &[ExecuteColumn],
    ) -> Vec<crate::pgwire::protocol::backend::FieldDescription> {
        columns
            .iter()
            .map(
                |column| crate::pgwire::protocol::backend::FieldDescription {
                    name: column.name.to_owned(),
                    data_type: column.data_type.to_owned().into(),
                },
            )
            .collect()
    }

    fn apply_statement_side_effects(&mut self, statement: &SQLStatement) {
        if let SQLStatement::Other(OtherStatement::UseDatabase(query)) = statement {
            self.engine.shared_state.client_info.database = query.database_name.clone();
        }
    }

    async fn step(
        &mut self,
        framed: &mut Framed<impl AsyncRead + AsyncWrite + Unpin, ConnectionCodec>,
    ) -> Result<Option<ConnectionState>, ConnectionError> {
        match self.state {
            ConnectionState::Startup => {
                match framed
                    .next()
                    .await
                    .ok_or(ConnectionError::ConnectionClosed)??
                {
                    ClientMessage::Startup(startup) => {
                        if let Some(database_name) = startup.parameters.get("database") {
                            // 해당 데이터베이스가 존재하는지 검사
                            let result = self
                                .engine
                                .shared_state
                                .engine
                                .find_database(database_name.clone())
                                .await;

                            match result {
                                Ok(has_match) => {
                                    if has_match {
                                        self.engine.shared_state.client_info.database =
                                            database_name.to_owned();

                                        log::debug!(
                                            "New Connection=> UUID:{} IP:{} DATABASE:{}",
                                            self.engine.shared_state.client_info.connection_id,
                                            self.engine.shared_state.client_info.ip,
                                            self.engine.shared_state.client_info.database
                                        );
                                    } else {
                                        return Err(ErrorResponse::fatal(
                                            SqlState::CONNECTION_EXCEPTION,
                                            format!("No database named '{}'", database_name),
                                        )
                                        .into());
                                    }
                                }
                                Err(error) => {
                                    return Err(ErrorResponse::fatal(
                                        SqlState::CONNECTION_EXCEPTION,
                                        format!("{:?}", error),
                                    )
                                    .into());
                                }
                            }
                        }
                    }
                    ClientMessage::SSLRequest => {
                        // we don't support SSL for now
                        // client will retry with startup packet
                        framed.send('N').await?;
                        return Ok(Some(ConnectionState::Startup));
                    }
                    ClientMessage::GSSENCRequest => {
                        // we don't support GSSAPI encryption for now
                        // libpq will retry with a regular startup packet
                        framed.send('N').await?;
                        return Ok(Some(ConnectionState::Startup));
                    }
                    _ => {
                        return Err(ErrorResponse::fatal(
                            SqlState::PROTOCOL_VIOLATION,
                            "expected startup message",
                        )
                        .into());
                    }
                }

                framed.feed(AuthenticationOk).await?;

                let param_statuses = &[
                    ("server_version", "13"),
                    ("server_encoding", "UTF8"),
                    ("client_encoding", "UTF8"),
                    ("DateStyle", "ISO"),
                    ("TimeZone", "UTC"),
                    ("integer_datetimes", "on"),
                ];

                for &(param, status) in param_statuses {
                    framed.feed(ParameterStatus::new(param, status)).await?;
                }

                framed.send(ReadyForQuery).await?;
                Ok(Some(ConnectionState::Idle))
            }
            ConnectionState::Idle => {
                let message = framed
                    .next()
                    .await
                    .ok_or(ConnectionError::ConnectionClosed)??;

                // Extended-protocol error recovery: PG spec requires the backend
                // to swallow every non-Sync message after an ErrorResponse until
                // it sees a Sync, and only then emit ReadyForQuery.
                if self.skip_until_sync {
                    match message {
                        ClientMessage::Sync => {
                            self.skip_until_sync = false;
                            framed.send(ReadyForQuery).await?;
                        }
                        ClientMessage::Terminate => return Ok(None),
                        _ => {}
                    }
                    return Ok(Some(ConnectionState::Idle));
                }

                self.current_message_is_simple =
                    matches!(&message, ClientMessage::Query(_));

                match message {
                    ClientMessage::Parse(parse) => {
                        let has_parameters = !parse.parameter_types.is_empty()
                            || Self::query_has_parameters(&parse.query);
                        let parsed_statement = if has_parameters {
                            None
                        } else {
                            self.parse_statement(&parse.query)?
                        };

                        self.statements.insert(
                            parse.prepared_statement_name,
                            PreparedStatement {
                                fields: match &parsed_statement {
                                    Some(statement) => self.engine.prepare(statement).await?,
                                    None => vec![],
                                },
                                statement: parsed_statement,
                                raw_query: if has_parameters {
                                    Some(parse.query)
                                } else {
                                    None
                                },
                                parameter_types: parse.parameter_types,
                            },
                        );
                        framed.send(ParseComplete).await?;
                    }
                    ClientMessage::Bind(bind) => {
                        let format_code = match bind.result_format {
                            BindFormat::All(format) => format,
                            BindFormat::PerColumn(_) => {
                                return Err(ErrorResponse::error(
                                    SqlState::FEATURE_NOT_SUPPORTED,
                                    "per-column format codes not supported",
                                )
                                .into());
                            }
                        };

                        let prepared = self
                            .prepared_statement(&bind.prepared_statement_name)?
                            .clone();

                        let prepared_statement = match prepared.statement {
                            Some(statement) => Some(statement),
                            None => match prepared.raw_query {
                                Some(query) => {
                                    match self
                                        .parse_parameterized_insert(&query, &bind.parameters)
                                        .await
                                    {
                                        Some(statement) => Some(statement),
                                        None => {
                                            let bound_query = Self::bind_query_parameters(
                                                &query,
                                                &bind.parameters,
                                            )?;
                                            self.parse_statement(&bound_query)?
                                        }
                                    }
                                }
                                None => None,
                            },
                        };

                        let portal = match prepared_statement {
                            Some(statement) => {
                                let portal = self.engine.create_portal(&statement).await?;
                                let fields = if Self::returns_rows(&statement) {
                                    prepared.fields.clone()
                                } else {
                                    Vec::new()
                                };
                                let row_desc = RowDescription {
                                    fields,
                                    format_code,
                                };

                                Some(BoundPortal {
                                    portal,
                                    row_desc,
                                    statement,
                                })
                            }
                            None => None,
                        };

                        self.portals.insert(bind.portal, portal);

                        framed.send(BindComplete).await?;
                    }
                    ClientMessage::Describe(Describe::PreparedStatement(ref statement_name)) => {
                        let prepared = self.prepared_statement(statement_name)?;
                        let fields = prepared.fields.clone();
                        let parameter_types = prepared.parameter_types.clone();
                        framed
                            .send(ParameterDescription { parameter_types })
                            .await?;
                        if fields.is_empty() {
                            framed.send(NoData).await?;
                        } else {
                            framed
                                .send(RowDescription {
                                    fields,
                                    format_code: FormatCode::Text,
                                })
                                .await?;
                        }
                    }
                    ClientMessage::Describe(Describe::Portal(ref portal_name)) => {
                        match self.portal(portal_name)? {
                            Some(portal) => {
                                if portal.row_desc.fields.is_empty() {
                                    framed.send(NoData).await?;
                                } else {
                                    framed.send(portal.row_desc.clone()).await?;
                                }
                            }
                            None => framed.send(NoData).await?,
                        }
                    }
                    ClientMessage::Close(Close::PreparedStatement(statement_name)) => {
                        self.statements.remove(&statement_name);
                        framed.send(CloseComplete).await?;
                    }
                    ClientMessage::Close(Close::Portal(portal_name)) => {
                        self.portals.remove(&portal_name);
                        framed.send(CloseComplete).await?;
                    }
                    ClientMessage::Flush => {
                        futures::SinkExt::<NoData>::flush(framed).await?;
                    }
                    ClientMessage::Sync => {
                        framed.send(ReadyForQuery).await?;
                    }
                    ClientMessage::Execute(exec) => match self.portal_mut(&exec.portal)? {
                        Some(bound) => {
                            let result = bound.portal.execute().await?;
                            let statement = bound.statement.clone();

                            if !bound.row_desc.fields.is_empty() {
                                let mut batch_writer = DataRowBatch::from_row_desc(&bound.row_desc);
                                bound.portal.fetch(&mut batch_writer).await?;
                                framed.send(batch_writer).await?;
                            }

                            framed
                                .send(CommandComplete {
                                    command_tag: Self::command_tag(&statement, &result),
                                })
                                .await?;

                            self.apply_statement_side_effects(&statement);
                        }
                        None => {
                            framed.send(EmptyQueryResponse).await?;
                        }
                    },
                    ClientMessage::Query(query) => {
                        if let Some(parsed) = self.parse_statement(&query)? {
                            let returns_rows = Self::returns_rows(&parsed);
                            let result = self.engine.execute_statement(&parsed).await?;

                            if returns_rows {
                                let row_desc = RowDescription {
                                    fields: Self::row_description_fields(&result.columns),
                                    format_code: FormatCode::Text,
                                };
                                let mut batch_writer = DataRowBatch::from_row_desc(&row_desc);
                                let mut portal = self.engine.create_portal(&parsed).await?;
                                portal.execute_result = Some(result.clone());
                                portal.fetch(&mut batch_writer).await?;
                                framed.send(row_desc).await?;
                                framed.send(batch_writer).await?;
                            }

                            framed
                                .send(CommandComplete {
                                    command_tag: Self::command_tag(&parsed, &result),
                                })
                                .await?;

                            self.apply_statement_side_effects(&parsed);
                        } else {
                            framed.send(EmptyQueryResponse).await?;
                        }
                        framed.send(ReadyForQuery).await?;
                    }
                    ClientMessage::Terminate => {
                        return Ok(None);
                    }
                    _ => {
                        return Err(ErrorResponse::error(
                            SqlState::PROTOCOL_VIOLATION,
                            "unexpected message",
                        )
                        .into());
                    }
                };

                Ok(Some(ConnectionState::Idle))
            }
        }
    }

    /// Given a stream (typically TCP), extract Postgres protocol messages and respond accordingly.
    /// This function only returns when the connection is closed (either gracefully or due to an error).
    pub async fn run(
        &mut self,
        stream: impl AsyncRead + AsyncWrite + Unpin,
    ) -> Result<(), ConnectionError> {
        let mut framed = Framed::new(stream, ConnectionCodec::new());

        loop {
            let new_state = match self.step(&mut framed).await {
                Ok(Some(state)) => state,
                Ok(None) => {
                    return Ok(());
                }
                Err(ConnectionError::ErrorResponse(err_info)) => {
                    framed.send(err_info.clone()).await?;

                    if err_info.severity == Severity::FATAL {
                        return Err(err_info.into());
                    }

                    if self.current_message_is_simple {
                        framed.send(ReadyForQuery).await?;
                    } else {
                        // Extended-protocol error: wait for Sync before ReadyForQuery.
                        self.skip_until_sync = true;
                    }
                    ConnectionState::Idle
                }
                Err(err) => {
                    framed
                        .send(ErrorResponse::fatal(
                            SqlState::CONNECTION_EXCEPTION,
                            "connection error",
                        ))
                        .await?;
                    return Err(err);
                }
            };

            self.state = new_state;
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum SqlSegment<'a> {
    Passthrough(&'a str),
    Placeholder(usize),
}

/// Splits SQL into pass-through spans and `$N` placeholder positions, respecting
/// single-quoted strings, quoted identifiers, line comments (`-- ...`), nested
/// block comments (`/* ... */`), and dollar-quoted strings (`$tag$ ... $tag$`).
/// Returns an error when a literal or comment is left unterminated.
fn scan_sql_segments(sql: &str) -> Result<Vec<SqlSegment<'_>>, ErrorResponse> {
    let bytes = sql.as_bytes();
    let mut segments = Vec::new();
    let mut passthrough_start = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        match bytes[i] {
            b'\'' => {
                i = skip_single_quoted(bytes, i)?;
            }
            b'"' => {
                i = skip_double_quoted(bytes, i)?;
            }
            b'-' if bytes.get(i + 1) == Some(&b'-') => {
                i = skip_line_comment(bytes, i);
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i = skip_block_comment(bytes, i)?;
            }
            b'$' => {
                if let Some(end) = try_skip_dollar_quoted(bytes, i)? {
                    i = end;
                } else if bytes.get(i + 1).is_some_and(u8::is_ascii_digit) {
                    if passthrough_start < i {
                        segments.push(SqlSegment::Passthrough(&sql[passthrough_start..i]));
                    }

                    let number_start = i + 1;
                    let mut j = number_start;
                    while j < bytes.len() && bytes[j].is_ascii_digit() {
                        j += 1;
                    }

                    let index: usize = sql[number_start..j].parse::<usize>().map_err(|error| {
                        ErrorResponse::error(SqlState::SYNTAX_ERROR, error.to_string())
                    })?;
                    segments.push(SqlSegment::Placeholder(index));
                    passthrough_start = j;
                    i = j;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }

    if passthrough_start < bytes.len() {
        segments.push(SqlSegment::Passthrough(&sql[passthrough_start..]));
    }

    Ok(segments)
}

fn skip_single_quoted(bytes: &[u8], start: usize) -> Result<usize, ErrorResponse> {
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b'\'' {
            if bytes.get(i + 1) == Some(&b'\'') {
                i += 2;
            } else {
                return Ok(i + 1);
            }
        } else {
            i += 1;
        }
    }
    Err(ErrorResponse::error(
        SqlState::SYNTAX_ERROR,
        "unterminated string literal",
    ))
}

fn skip_double_quoted(bytes: &[u8], start: usize) -> Result<usize, ErrorResponse> {
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            if bytes.get(i + 1) == Some(&b'"') {
                i += 2;
            } else {
                return Ok(i + 1);
            }
        } else {
            i += 1;
        }
    }
    Err(ErrorResponse::error(
        SqlState::SYNTAX_ERROR,
        "unterminated quoted identifier",
    ))
}

fn skip_line_comment(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 2;
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    i
}

fn skip_block_comment(bytes: &[u8], start: usize) -> Result<usize, ErrorResponse> {
    let mut i = start + 2;
    let mut depth: u32 = 1;
    while i + 1 < bytes.len() {
        if bytes[i] == b'/' && bytes[i + 1] == b'*' {
            depth += 1;
            i += 2;
        } else if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return Ok(i);
            }
        } else {
            i += 1;
        }
    }
    Err(ErrorResponse::error(
        SqlState::SYNTAX_ERROR,
        "unterminated block comment",
    ))
}

/// If `bytes[start]` is `$` and starts a valid dollar-quoted string, returns the
/// index just past the closing tag. Returns `Ok(None)` when the sequence is not
/// a dollar-quoted literal (so the caller can treat `$N` as a placeholder).
fn try_skip_dollar_quoted(bytes: &[u8], start: usize) -> Result<Option<usize>, ErrorResponse> {
    let mut i = start + 1;
    while i < bytes.len() && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
        i += 1;
    }
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }

    if bytes.get(i) != Some(&b'$') {
        return Ok(None);
    }

    let tag = &bytes[start..=i];
    let mut j = i + 1;
    while j + tag.len() <= bytes.len() {
        if &bytes[j..j + tag.len()] == tag {
            return Ok(Some(j + tag.len()));
        }
        j += 1;
    }

    Err(ErrorResponse::error(
        SqlState::SYNTAX_ERROR,
        "unterminated dollar-quoted string",
    ))
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};
    use std::path::PathBuf;
    use std::sync::Arc;

    use tokio::sync::Mutex;

    use crate::config::launch_config::LaunchConfig;
    use crate::engine::DBEngine;
    use crate::engine::ast::dml::insert::InsertData;
    use crate::engine::ast::types::SQLExpression;
    use crate::engine::ast::{DMLStatement, SQLStatement};
    use crate::engine::parser::context::ParserContext;
    use crate::engine::parser::predule::Parser;
    use crate::engine::server::client::ClientInfo;
    use crate::engine::server::shared_state::SharedState;
    use crate::engine::wal::endec::implements::bincode::{BincodeDecoder, BincodeEncoder};
    use crate::engine::wal::manager::builder::WALBuilder;

    use super::Connection;

    #[test]
    fn bind_query_parameters_quotes_and_escapes_text_values() {
        let query = "insert into key_value (key, value) values ($1, $2)";
        let bound = Connection::bind_query_parameters(
            query,
            &[Some("a'b".to_string()), Some("value".to_string())],
        )
        .unwrap();

        assert_eq!(
            bound,
            "insert into key_value (key, value) values ('a''b', 'value')"
        );
    }

    #[test]
    fn bind_query_parameters_keeps_placeholders_inside_string_literals() {
        let query = "select '$1', $1";
        let bound = Connection::bind_query_parameters(query, &[Some("real".to_string())]).unwrap();

        assert_eq!(bound, "select '$1', 'real'");
    }

    #[test]
    fn bind_query_parameters_preserves_utf8_query_text() {
        let query = "insert into 한글_table (이름) values ($1)";
        let bound = Connection::bind_query_parameters(query, &[Some("값".to_string())]).unwrap();

        assert_eq!(bound, "insert into 한글_table (이름) values ('값')");
    }

    #[test]
    fn query_has_parameters_ignores_placeholders_inside_string_literals() {
        assert!(!Connection::query_has_parameters("select '$1'"));
        assert!(Connection::query_has_parameters("select '$1', $1"));
    }

    #[test]
    fn query_has_parameters_ignores_placeholders_inside_line_comments() {
        assert!(!Connection::query_has_parameters("select 1 -- $1"));
        assert!(!Connection::query_has_parameters("-- $1\nselect 1"));
        assert!(Connection::query_has_parameters("-- $1\nselect $1"));
    }

    #[test]
    fn query_has_parameters_ignores_placeholders_inside_block_comments() {
        assert!(!Connection::query_has_parameters("select 1 /* $1 */"));
        assert!(!Connection::query_has_parameters("/* nested /* $1 */ */ select 1"));
        assert!(Connection::query_has_parameters("/* $1 */ select $1"));
    }

    #[test]
    fn query_has_parameters_ignores_placeholders_inside_dollar_quoted_strings() {
        assert!(!Connection::query_has_parameters("select $body$ $1 $body$"));
        assert!(!Connection::query_has_parameters("select $$ $1 $$"));
        assert!(Connection::query_has_parameters("select $body$ x $body$, $1"));
    }

    #[test]
    fn query_has_parameters_ignores_placeholders_inside_quoted_identifiers() {
        assert!(!Connection::query_has_parameters("select \"$1\""));
        assert!(Connection::query_has_parameters("select \"col\", $1"));
    }

    #[test]
    fn bind_query_parameters_leaves_line_comment_body_untouched() {
        let query = "select $1 -- ignore $1 in comment";
        let bound = Connection::bind_query_parameters(query, &[Some("v".to_string())]).unwrap();
        assert_eq!(bound, "select 'v' -- ignore $1 in comment");
    }

    #[test]
    fn bind_query_parameters_leaves_block_comment_body_untouched() {
        let query = "select /* $1 */ $1";
        let bound = Connection::bind_query_parameters(query, &[Some("v".to_string())]).unwrap();
        assert_eq!(bound, "select /* $1 */ 'v'");
    }

    #[test]
    fn bind_query_parameters_leaves_dollar_quoted_body_untouched() {
        let query = "select $body$ inner $1 $body$, $1";
        let bound = Connection::bind_query_parameters(query, &[Some("v".to_string())]).unwrap();
        assert_eq!(bound, "select $body$ inner $1 $body$, 'v'");
    }

    #[test]
    fn bind_query_parameters_leaves_quoted_identifier_body_untouched() {
        let query = "select \"$1 column\", $1";
        let bound = Connection::bind_query_parameters(query, &[Some("v".to_string())]).unwrap();
        assert_eq!(bound, "select \"$1 column\", 'v'");
    }

    #[test]
    fn bind_query_parameters_rejects_unterminated_block_comment() {
        let err = Connection::bind_query_parameters("select 1 /* $1 ", &[Some("v".to_string())]);
        assert!(err.is_err());
    }

    fn parse_statement(sql: &str) -> SQLStatement {
        let mut parser = Parser::with_string(sql.to_owned()).unwrap();
        parser
            .parse(ParserContext::default().set_default_database("rrdb".to_string()))
            .unwrap()
            .remove(0)
    }

    async fn build_test_connection(test_name: &str) -> Connection {
        let base_path = PathBuf::from("target").join(test_name);
        if base_path.exists() {
            tokio::fs::remove_dir_all(&base_path).await.unwrap();
        }

        let config = LaunchConfig::default_for_base_path(&base_path);
        tokio::fs::create_dir_all(&config.data_directory)
            .await
            .unwrap();
        tokio::fs::create_dir_all(&config.wal_directory)
            .await
            .unwrap();
        let wal = WALBuilder::new(&config)
            .build(BincodeDecoder::new(), BincodeEncoder::new())
            .await
            .unwrap();
        Connection::new(SharedState {
            engine: Arc::new(DBEngine::new(config)),
            wal_manager: Arc::new(Mutex::new(wal)),
            client_info: ClientInfo {
                ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
                connection_id: "test".to_string(),
                database: "rrdb".to_string(),
            },
        })
    }

    async fn execute_sql(connection: &Connection, sql: &str) {
        connection
            .engine
            .shared_state
            .engine
            .process_query(
                parse_statement(sql),
                connection.engine.shared_state.wal_manager.clone(),
                connection
                    .engine
                    .shared_state
                    .client_info
                    .connection_id
                    .clone(),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn parse_parameterized_insert_builds_insert_statement_without_sql_reparse() {
        let connection = build_test_connection("test_pgwire_parameterized_insert").await;
        execute_sql(&connection, "create database rrdb").await;
        execute_sql(
            &connection,
            "create table key_value (key varchar(255), value varchar(255))",
        )
        .await;

        let statement = connection
            .parse_parameterized_insert(
                "INSERT INTO key_value (key, value) VALUES ($1, $2)",
                &[Some("k1".to_string()), Some("v1".to_string())],
            )
            .await
            .unwrap();

        let insert = match statement {
            crate::engine::ast::SQLStatement::DML(DMLStatement::InsertQuery(insert)) => insert,
            other => panic!("expected insert statement, got {other:?}"),
        };

        assert_eq!(insert.into_table.unwrap().table_name, "key_value");
        assert_eq!(insert.columns, vec!["key", "value"]);
    }

    #[tokio::test]
    async fn parse_parameterized_insert_rejects_multirow_fast_path() {
        let connection = build_test_connection("test_pgwire_parameterized_insert_multirow").await;

        let statement = connection
            .parse_parameterized_insert(
                "INSERT INTO key_value (key, value) VALUES ($1, $2), ($3, $4)",
                &[
                    Some("k1".to_string()),
                    Some("v1".to_string()),
                    Some("k2".to_string()),
                    Some("v2".to_string()),
                ],
            )
            .await;

        assert!(statement.is_none());
    }

    #[tokio::test]
    async fn parse_parameterized_insert_uses_target_column_types() {
        let connection = build_test_connection("test_pgwire_parameterized_insert_types").await;
        execute_sql(&connection, "create database rrdb").await;
        execute_sql(
            &connection,
            "create table typed_values (id int, enabled boolean, ratio float, label varchar(255))",
        )
        .await;

        let statement = connection
            .parse_parameterized_insert(
                "INSERT INTO typed_values (id, enabled, ratio, label) VALUES ($1, $2, $3, $4)",
                &[
                    Some("42".to_string()),
                    Some("true".to_string()),
                    Some("3.5".to_string()),
                    Some("007".to_string()),
                ],
            )
            .await
            .unwrap();

        let insert = match statement {
            crate::engine::ast::SQLStatement::DML(DMLStatement::InsertQuery(insert)) => insert,
            other => panic!("expected insert statement, got {other:?}"),
        };
        let InsertData::Values(values) = insert.data else {
            panic!("expected insert values");
        };
        let values = &values[0].list;

        assert_eq!(values[0], Some(SQLExpression::Integer(42)));
        assert_eq!(values[1], Some(SQLExpression::Boolean(true)));
        assert_eq!(values[2], Some(SQLExpression::Float(3.5)));
        assert_eq!(values[3], Some(SQLExpression::String("007".to_string())));
    }

    use bytes::{BufMut, BytesMut};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn startup_bytes(database: &str) -> BytesMut {
        let mut body = BytesMut::new();
        body.put_i16(3);
        body.put_i16(0);
        body.put_slice(b"user\0me\0");
        if !database.is_empty() {
            body.put_slice(b"database\0");
            body.put_slice(database.as_bytes());
            body.put_u8(0);
        }
        body.put_u8(0);

        let mut msg = BytesMut::new();
        msg.put_i32((4 + body.len()) as i32);
        msg.extend_from_slice(&body);
        msg
    }

    fn bind_bytes(portal: &str, statement: &str) -> BytesMut {
        let mut body = BytesMut::new();
        body.put_slice(portal.as_bytes());
        body.put_u8(0);
        body.put_slice(statement.as_bytes());
        body.put_u8(0);
        body.put_i16(0); // param format codes
        body.put_i16(0); // param values
        body.put_i16(0); // result format codes

        let mut msg = BytesMut::new();
        msg.put_u8(b'B');
        msg.put_i32((4 + body.len()) as i32);
        msg.extend_from_slice(&body);
        msg
    }

    fn execute_bytes(portal: &str) -> BytesMut {
        let mut body = BytesMut::new();
        body.put_slice(portal.as_bytes());
        body.put_u8(0);
        body.put_i32(0);

        let mut msg = BytesMut::new();
        msg.put_u8(b'E');
        msg.put_i32((4 + body.len()) as i32);
        msg.extend_from_slice(&body);
        msg
    }

    fn tag_only_bytes(tag: u8) -> BytesMut {
        let mut msg = BytesMut::new();
        msg.put_u8(tag);
        msg.put_i32(4);
        msg
    }

    fn simple_query_bytes(sql: &str) -> BytesMut {
        let mut body = BytesMut::new();
        body.put_slice(sql.as_bytes());
        body.put_u8(0);

        let mut msg = BytesMut::new();
        msg.put_u8(b'Q');
        msg.put_i32((4 + body.len()) as i32);
        msg.extend_from_slice(&body);
        msg
    }

    async fn read_message<R: tokio::io::AsyncRead + Unpin>(reader: &mut R) -> (u8, Vec<u8>) {
        let mut header = [0u8; 5];
        reader.read_exact(&mut header).await.unwrap();
        let length = i32::from_be_bytes(header[1..5].try_into().unwrap()) as usize;
        let mut body = vec![0u8; length - 4];
        if !body.is_empty() {
            reader.read_exact(&mut body).await.unwrap();
        }
        (header[0], body)
    }

    async fn drain_startup_replies<R: tokio::io::AsyncRead + Unpin>(reader: &mut R) {
        // AuthenticationOk, then ParameterStatus messages, then ReadyForQuery.
        loop {
            let (tag, _) = read_message(reader).await;
            if tag == b'Z' {
                return;
            }
        }
    }

    /// PostgreSQL extended-protocol spec: after an ErrorResponse, the backend
    /// must silently discard every message until it sees Sync, and only then
    /// send ReadyForQuery.
    #[tokio::test]
    async fn extended_protocol_error_defers_ready_for_query_until_sync() {
        let (mut client, server) = tokio::io::duplex(4096);
        let mut connection = build_test_connection("test_pgwire_skip_until_sync").await;

        let server_task = tokio::spawn(async move {
            let _ = connection.run(server).await;
        });

        client.write_all(&startup_bytes("")).await.unwrap();
        drain_startup_replies(&mut client).await;

        // Bind referencing a statement that was never Parsed → ErrorResponse.
        client
            .write_all(&bind_bytes("p1", "nonexistent"))
            .await
            .unwrap();
        // Follow-up Execute must be discarded silently.
        client.write_all(&execute_bytes("p1")).await.unwrap();
        // Sync ends the recovery window and triggers ReadyForQuery.
        client.write_all(&tag_only_bytes(b'S')).await.unwrap();

        let (tag, _) = read_message(&mut client).await;
        assert_eq!(tag, b'E', "expected ErrorResponse first");

        let (tag, _) = read_message(&mut client).await;
        assert_eq!(
            tag, b'Z',
            "next message must be ReadyForQuery (after Sync), not a response to the discarded Execute"
        );

        client.write_all(&tag_only_bytes(b'X')).await.unwrap();
        let _ = server_task.await;
    }

    /// Simple query (`Q`) mode still emits ReadyForQuery immediately after an
    /// ErrorResponse — the extended-protocol recovery window applies only to
    /// Parse/Bind/Describe/Execute/Close/Flush.
    #[tokio::test]
    async fn simple_query_error_still_emits_ready_for_query_immediately() {
        let (mut client, server) = tokio::io::duplex(4096);
        let mut connection = build_test_connection("test_pgwire_simple_query_error").await;

        let server_task = tokio::spawn(async move {
            let _ = connection.run(server).await;
        });

        client.write_all(&startup_bytes("")).await.unwrap();
        drain_startup_replies(&mut client).await;

        // Syntax error in simple query mode (SELECT with nothing after it).
        client
            .write_all(&simple_query_bytes("select"))
            .await
            .unwrap();

        let (tag, _) = read_message(&mut client).await;
        assert_eq!(tag, b'E', "expected ErrorResponse");

        let (tag, _) = read_message(&mut client).await;
        assert_eq!(tag, b'Z', "simple query error must be followed by ReadyForQuery without Sync");

        client.write_all(&tag_only_bytes(b'X')).await.unwrap();
        let _ = server_task.await;
    }
}
