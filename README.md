# rrdb

![](https://img.shields.io/badge/language-Rust-red) ![](https://img.shields.io/badge/version-0.0.0%20alpha-brightgreen) [![GitHub license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/myyrakle/rrdb/blob/master/LICENSE)

Rust-based RDB

not complete

### Server

```
# 스토리지 초기화
cargo run --bin rrdb init
# 서버 실행
cargo run --bin rrdb run
```

### Client

```
psql -U rrdb -p 55555
```

### Syntax

#### Database

```
# 데이터베이스 리스트업
SHOW DATABASES;
```

```
# 데이터베이스 생성
CREATE DATABASE "database name";
```

```
# 데이터베이스 삭제
DROP DATABASE "database name";
```

```
# 데이터베이스 변경
ALTER DATABASE "from name" rename to "to name";
```

```
# 데이터베이스 변경
USE "database name";
or
\c "database name";
```
