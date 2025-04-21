# 言語仕様概要

以下では、文法と使用例をより厳密に定義します。

## 文法

### 字句要素（Tokens）

- コメント
  - 行コメント: `// ...`
  - ブロックコメント: `/* ... */`
- 識別子: `[A-Za-z_][A-Za-z0-9_]*`
- キーワード: `fn`, `let`, `if`, `else`, `while`, `for`, `in`, `return`, `mod`, `true`, `false`
- リテラル
  - 整数: `[0-9]+`
  - 浮動小数点: `[0-9]+\.[0-9]+`
  - 文字列: `"(\\.|[^"\\])*"`
- 記号: `(`, `)`, `{`, `}`, `,`, `;`, `+`, `-`, `*`, `/`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `=`

### 基本型

- 真偽値: `bool`
- 符号付き整数値: `s32`, `s64`
- 符号なし整数値: `u32`, `u64`
- 浮動小数点: `f32`, `f64`
- 文字（Unocode scalar value）: `char`
- 文字列: `string`

### EBNF 文法（抜粋）

```ebnf
Program       = { ModuleDecl | Statement } ;
ModuleDecl    = "mod" Identifier "{" { Statement } "}" ;
Statement     = VariableDecl ";"
              | FunctionDecl
              | ExprStmt ";"
              | IfStmt
              | WhileStmt
              | ForStmt
              | ReturnStmt ";" ;
VariableDecl  = "let" Identifier [ ":" Type ] [ "=" Expr ] ;
FunctionDecl  = "fn" Identifier "(" [ ParamList ] ")" [ "->" Type ] Block ;
ParamList     = Param { "," Param } ;
Param         = Identifier ":" Type ;
IfStmt        = "if" "(" Expr ")" Block [ "else" ( Block | IfStmt ) ] ;
WhileStmt     = "while" "(" Expr ")" Block ;
ForStmt       = "for" "(" Identifier "in" Expr ")" Block ;
ReturnStmt    = "return" [ Expr ] ;
ExprStmt      = Expr ;
Block         = "{" { Statement } "}" ;
Expr          = Assignment ;
Assignment    = LogicalOr [ "=" Assignment ] ;
LogicalOr     = LogicalAnd { "||" LogicalAnd } ;
LogicalAnd    = Equality { "&&" Equality } ;
Equality      = Comparison { ( "==" | "!=" ) Comparison } ;
Comparison    = Addition { ( "<" | "<=" | ">" | ">=" ) Addition } ;
Addition      = Multiplication { ( "+" | "-" ) Multiplication } ;
Multiplication= Unary { ( "*" | "/" ) Unary } ;
Unary         = [ ( "!" | "-" ) ] Primary ;
Primary       = Literal
              | Identifier
              | "(" Expr ")"
              | FunctionCall ;
FunctionCall  = Identifier "(" [ ArgList ] ")" ;
ArgList       = Expr { "," Expr } ;
Type          = "i32" | "f64" | "bool" | "string" | Identifier ;
Literal       = IntegerLiteral | FloatLiteral | StringLiteral | BooleanLiteral ;
```

### 例

```ai
mod math {
    fn add(a: i32, b: i32) -> i32 {
        return a + b;
    }
}

fn main() {
    let x: i32 = 10;
    let y = 20;             // 型推論 (i32)
    let z = math::add(x,y);
    if (z > 25) {
        print("greater\n");
    } else {
        print("small\n");
    }
    for (i in 0..5) {
        print(i);
    }
    while (x < y) {
        x = x + 1;
    }
}
```
