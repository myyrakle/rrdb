## AST (Abstract Syntax Tree)

- SQL 쿼리에 대한 추상 구문 트리(AST) 정의입니다.
- [구문 분석기(Parser)](./../parser/README.md)에 의해서 생성됩니다.

### 소스코드

- DCL, DDL, DML별로 분리가 되어있습니다.
- 공용 트레잇은 [traits](./traits/READ) 모듈에, 공용 타입은 [types](./types/README.md) 모듈에 존재합니다.
