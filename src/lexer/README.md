## lexer

어휘(토큰) 및 어휘 분석기입니다.
텍스트를 유효한 단어로 구분합니다.

예를 들어 "select age+1 from person"와 같은 문장은
["select", "age", "+", "1", "from", "person"]으로 분리되어야 합니다.
lexer는 이에 대한 처리를 주관합니다.

### 소스코드

어휘에 대한 정의는 [tokens.rs](./tokens.rs)에,
어휘 분석 로직은 [tokenizer.rs](./tokenizer.rs)에 있습니다.
