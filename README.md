# rrdb

![](https://img.shields.io/badge/language-Rust-red) ![](https://img.shields.io/badge/version-0.0.1%20alpha-brightgreen) [![GitHub license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/myyrakle/rrdb/blob/master/LICENSE)

Rust-based RDB

## not complete

---

### 설치

cargo를 사용한다.

```
cargo install rrdb
```

- 플랫폼별 초기화 (Linux)

심볼릭 링크를 생성하고 초기화를 수행합니다.

```
sudo ln -s /home/$USER/.cargo/bin/rrdb /usr/bin/rrdb
sudo rrdb init
```

- 플랫폼별 초기화 (MacOS)

심볼릭 링크를 생성하고 초기화를 수행합니다.

```
sudo ln -s /home/$USER/.cargo/bin/rrdb /usr/local/bin/rrdb
sudo rrdb init
```

- 플랫폼별 초기화 (Windows)

powershell을 관리자 권한으로 실행하고 다음 명령어를 수행합니다.

```
mkdir 'C:\Program Files\rrdb'
cp ~/.cargo/bin/rrdb.exe 'C:\Program Files\rrdb\'
'C:\Program Files\rrdb\rrdb.exe' init
```

---

### 기본 사용법

#### Server

```
# 스토리지 초기화
cargo run --bin rrdb init
# 서버 실행
cargo run --bin rrdb run
```

#### Client

```
psql -U rrdb -p 22208 --host 0.0.0.0
```

---

### Syntax

1. 키워드는 대소문자를 구별하지 않습니다.
2. 문자열은 작은 따옴표(')로 구분되며, 따옴표를 포함시킬 때는 2개를 겹칩니다.
3. 식별자는 단순 텍스트르로 구성해도 되고, 큰 따옴표(")로 구분해도 됩니다.

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

#### Table

```
# 테이블 목록 조회
SHOW TABLES
```

```
# 테이블 상세정보 조회
DESC "table name"
```

```
# 테이블 생성
# (table_constraint는 차후 추가할 예정입니다.)
CREATE TABLE [ IF NOT EXISTS ] "table name"
(
    [
        {
            "column name" data_type  [ column_constraint [ ... ] ]
        }
        [, ... ]
    ]
)

# column_constraint는 아래 형태 중 하나입니다.
# (CONSTRAINT나 CHECK, UNIQUE, REFERENCES 등은 차후 추가할 예정입니다.)
{
    NOT NULL |
    NULL |
    DEFAULT default_expr |
    PRIMARY KEY index_parameters
}
```

```
# 테이블 수정

1. ALTER TABLE [ IF EXISTS ] name
    action
2. ALTER TABLE [ IF EXISTS ] name
    RENAME [ COLUMN ] column_name TO new_column_name
3. ALTER TABLE [ IF EXISTS ] name
    RENAME TO new_name

# action은 다음 중 하나입니다.

1. ADD [ COLUMN ] column_name data_type [ column_constraint [ ... ] ] # 향후 [IF NOT EXISTS] 신택스 추가 필요
2. DROP [ COLUMN ]  column_name # 향후 [ IF EXISTS ] 신택스 추가 필요
3. ALTER [ COLUMN ] column_name [ SET DATA ] TYPE data_type
4. ALTER [ COLUMN ] column_name SET DEFAULT expression
5. ALTER [ COLUMN ] column_name DROP DEFAULT
6. ALTER [ COLUMN ] column_name { SET | DROP } NOT NULL
```

#### Insert

```
INSERT INTO table_name ( column_name [, ...] )
{
    VALUES ( { expression | DEFAULT } [, ...] ) [, ...]
    |
    select_query
}
[, ...] ]
```

#### Select

```
SELECT
    [ * | expression [ [ AS ] output_name ] [, ...] ]
[ FROM from_item [, ...] ]
[ WHERE condition ]
[ GROUP BY grouping_element [, ...] ]
[ HAVING condition ]
[ ORDER BY expression [ ASC | DESC ] [ NULLS { FIRST | LAST } ] [, ...] ]
[ LIMIT limit_number ]
[ OFFSET offset_number ]

from_item은 다음 중 하나입니다.
1. table_name  [ [ AS ] alias ]
2. ( select ) [ AS ] alias
```

#### Update

```
UPDATE table_name
SET { column_name = { expression } } [, ...]
[ WHERE condition ]
```

#### Delete

```
DELETE FROM table_name
[ WHERE condition ]
```
