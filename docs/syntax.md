# Yuni言語 構文仕様

Yuni言語は、Rust、Go、C++の良い部分を組み合わせ、メモリ安全性とパフォーマンスを両立させた現代的なシステムプログラミング言語です。

## プログラム構造

### パッケージ宣言

すべてのYuniプログラムは`package`宣言で始まります：

```yuni
package main
```

### インポート文

外部モジュールやライブラリをインポートします：

```yuni
import (
    "math"
    "fmt" as format
)
```

## 基本構文

### コメント

```yuni
// 行コメント

/*
  ブロックコメント
  複数行にわたって
  記述できます
*/
```

### 識別子

識別子は文字またはアンダースコアで始まり、その後に文字、数字、アンダースコアを続けることができます：

- 有効: `identifier`, `_private`, `camelCase`, `snake_case`, `function1`
- 無効: `1invalid`, `kebab-case`

## データ型

### 基本型

#### 整数型

```yuni
// 符号付き整数
let a: i8 = -128;     // 8ビット符号付き整数
let b: i16 = 1000;    // 16ビット符号付き整数  
let c: i32 = 42;      // 32ビット符号付き整数（デフォルト）
let d: i64 = 1000000; // 64ビット符号付き整数
let e: i128 = 42;     // 128ビット符号付き整数
let f: i256 = 42;     // 256ビット符号付き整数

// 符号無し整数
let g: u8 = 255;      // 8ビット符号無し整数
let h: u16 = 65535;   // 16ビット符号無し整数
let i: u32 = 42;      // 32ビット符号無し整数
let j: u64 = 1000000; // 64ビット符号無し整数
let k: u128 = 42;     // 128ビット符号無し整数
let l: u256 = 42;     // 256ビット符号無し整数
```

#### 浮動小数点数型

```yuni
let a: f8 = 3.14;     // 8ビット浮動小数点数
let b: f16 = 2.718;   // 16ビット浮動小数点数
let c: f32 = 1.414;   // 32ビット浮動小数点数
let d: f64 = 3.14159; // 64ビット浮動小数点数（デフォルト）
```

#### その他の基本型

```yuni
let flag: bool = true;           // 真偽値
let message: String = "Hello";   // 文字列
let nothing: void = ();          // void型（ユニット型）
```

### 複合型

#### 配列

```yuni
let numbers: [i32] = [1, 2, 3, 4, 5];
let floats: [f64] = [1.0, 2.0, 3.0];
let empty: [i32] = [];
```

#### タプル

```yuni
let point: (f64, f64) = (3.0, 4.0);
let person: (String, i32, bool) = ("Alice", 30, true);
let empty: () = ();
```

#### 参照型

```yuni
let value: i32 = 42;
let reference: &i32 = &value;      // 不変参照
let mut_value: mut i32 = 42;
let mut_ref: &mut i32 = &mut_value; // 可変参照
```

### ユーザー定義型

#### 構造体

```yuni
type Point struct {
    x: f64,
    y: f64
}

type Person struct {
    name: String,
    age: i32,
    is_student: bool
}

// 構造体リテラル
let p = Point { x: 3.0, y: 4.0 };
let person = Person { 
    name: "Alice", 
    age: 25, 
    is_student: true 
};
```

#### 代数的データ型（Enum）

```yuni
type State enum {
    Initial,
    Running(count: i32),
    Finished,
    Error(message: String)
}

// 使用例
let state = State::Running(42);
let error = State::Error("Something went wrong");
```

## 変数と定数

### 変数宣言

```yuni
let x: i32 = 42;           // 不変変数（デフォルト）
let mut y: i32 = 10;       // 可変変数

// 型推論
let a = 42;                // i32として推論
let b = 3.14;              // f64として推論
let c = "Hello";           // Stringとして推論
```

### 代入

```yuni
let mut x = 10;
x = 20;           // 基本代入
x += 5;           // 複合代入演算子
x -= 3;
x *= 2;
x /= 4;
x %= 3;
```

### 分割代入

```yuni
// タプル分割代入
let (x, y) = (10, 20);
let (a, b, c) = (1, 2, 3);

// 構造体分割代入
let Point { x, y } = point;
let Person { name, age, is_student } = person;
```

## リテラル

### 数値リテラル

```yuni
let decimal = 42;           // 10進数
let hex = 0xFF;             // 16進数
let octal = 0o755;          // 8進数
let binary = 0b1010;        // 2進数

// 型サフィックス
let typed_int = 42i32;      // i32として明示
let typed_float = 3.14f32;  // f32として明示

// アンダースコア区切り（可読性向上）
let large_number = 1_000_000;
let float_number = 3.141_592;
```

### 文字列リテラル

```yuni
let simple = "Hello, World!";
let with_escape = "Line 1\nLine 2\tTabbed";

// テンプレート文字列（文字列補間）
let name = "Alice";
let age = 25;
let message = `Hello, ${name}! You are ${age} years old.`;
let calculation = `The result is ${2 + 3}`;
```

### 真偽値リテラル

```yuni
let is_true = true;
let is_false = false;
```

## 演算子

### 算術演算子

```yuni
let a = 10;
let b = 3;

let sum = a + b;        // 加算: 13
let diff = a - b;       // 減算: 7  
let product = a * b;    // 乗算: 30
let quotient = a / b;   // 除算: 3（整数除算）
let remainder = a % b;  // 剰余: 1
```

### 比較演算子

```yuni
let a = 10;
let b = 20;

let equal = a == b;     // 等価: false
let not_equal = a != b; // 非等価: true
let less = a < b;       // 未満: true
let greater = a > b;    // より大きい: false
let less_eq = a <= b;   // 以下: true
let greater_eq = a >= b; // 以上: false
```

### 論理演算子

```yuni
let p = true;
let q = false;

let and_result = p && q;  // 論理積: false
let or_result = p || q;   // 論理和: true
let not_result = !p;      // 否定: false
```

### 参照演算子

```yuni
let value = 42;
let reference = &value;     // 参照取得
let mut_value = mut 42;
let mut_ref = &mut mut_value; // 可変参照取得
let dereferenced = *reference; // 参照外し
```

### 代入演算子

```yuni
let mut x = 10;

x += 5;   // x = x + 5
x -= 3;   // x = x - 3
x *= 2;   // x = x * 2
x /= 4;   // x = x / 4
x %= 3;   // x = x % 3
```

## 制御構造

### 条件分岐

#### if文

```yuni
let x = 10;

if x > 0 {
    println("正の数");
} else if x < 0 {
    println("負の数");
} else {
    println("ゼロ");
}

// if式（値を返す）
let result = if x > 0 { "positive" } else { "non-positive" };
```

### ループ

#### whileループ

```yuni
let mut i = 0;
while i < 10 {
    println("i: ", i);
    i += 1;
}
```

#### forループ

```yuni
// C言語スタイルのforループ
for (let mut i = 0; i < 10; i += 1) {
    println("i: ", i);
}
```

### ブロック

```yuni
{
    let x = 10;
    let y = 20;
    println("x + y = ", x + y);
    // xとyはここでスコープを抜ける
}
```

## 関数

### 関数定義

```yuni
// 基本的な関数
fn greet(name: String) {
    println("Hello, ", name, "!");
}

// 戻り値を持つ関数
fn add(a: i32, b: i32): i32 {
    return a + b;
}

// 式として戻り値を返す（returnなし）
fn multiply(a: i32, b: i32): i32 {
    a * b
}

// 複数の戻り値（タプル）
fn divide_with_remainder(a: i32, b: i32): (i32, i32) {
    return (a / b, a % b);
}
```

### メソッド

```yuni
type Point struct {
    x: f64,
    y: f64
}

// 不変参照を受け取るメソッド
fn (p: &Point) distance_from_origin(): f64 {
    return math.sqrt(p.x * p.x + p.y * p.y);
}

// 可変参照を受け取るメソッド
fn (p: &mut Point) move_by(dx: f64, dy: f64) {
    p.x += dx;
    p.y += dy;
}

// 値を受け取るメソッド（所有権の移動）
fn (p: Point) into_tuple(): (f64, f64) {
    return (p.x, p.y);
}
```

### 関数呼び出し

```yuni
let result = add(10, 20);
let (quotient, remainder) = divide_with_remainder(17, 5);

let point = Point { x: 3.0, y: 4.0 };
let distance = point.distance_from_origin();
```

## 式と文

### 式（Expression）

式は値を評価します：

```yuni
let x = 42;              // 42は式
let y = x + 10;          // x + 10は式
let z = if x > 0 { 1 } else { -1 }; // if式
let result = {           // ブロック式
    let a = 10;
    let b = 20;
    a + b                // 最後の式が値となる
};
```

### 文（Statement）

文は動作を実行します：

```yuni
let x = 42;              // let文
x += 10;                 // 代入文
println("Hello");        // 関数呼び出し文
if x > 0 {              // if文
    println("positive");
}
```

## 型注釈とキャスト

### 型注釈

```yuni
let x: i32 = 42;               // 明示的型注釈
let y = 42i32;                 // サフィックスによる型指定
let z: f64 = 3.14;             // 浮動小数点数の型注釈
```

### 型キャスト

```yuni
let x: i32 = 42;
let y: f64 = x as f64;         // i32からf64へのキャスト
let z: i64 = y as i64;         // f64からi64へのキャスト
```

## フィールドアクセスと配列アクセス

### フィールドアクセス

```yuni
let point = Point { x: 3.0, y: 4.0 };
let x_value = point.x;         // フィールドアクセス
```

### 配列アクセス

```yuni
let numbers = [1, 2, 3, 4, 5];
let first = numbers[0];        // 配列の最初の要素
let third = numbers[2];        // 配列の3番目の要素
```

## ライフタイム注釈

関数の戻り値が引数のライフタイムに依存する場合：

```yuni
fn new(message: &String): (ret: Messenger)
lives
    ret = message
{
    return Messenger { message };
}
```

## 自動参照取得

Yuniは多くの場面で自動的に参照を取得します：

```yuni
fn show(val: &i32) {
    println("value: ", val);
}

fn main() {
    let value = 128;
    show(value);                // 自動的に&valueとして渡される
    
    let ref: &i32 = value;      // 自動的に参照を取得
}
```

## 組み込み関数

### println

```yuni
println();                     // 空行を出力
println("Hello, World!");      // 文字列を出力
println("Value: ", 42);        // 複数の引数（スペース区切り）
println("x: ", x, ", y: ", y); // 変数と文字列の混合
```

## コメント規約

```yuni
/// この関数は2つの数値を加算します
/// 
/// 引数:
/// - a: 最初の数値
/// - b: 2番目の数値
/// 
/// 戻り値:
/// - 2つの数値の和
fn add(a: i32, b: i32): i32 {
    return a + b;
}
```

## ベストプラクティス

1. **命名規約**: snake_caseを関数名と変数名に、PascalCaseを型名に使用
2. **型注釈**: 型が自明でない場合は明示的に注釈を付ける
3. **参照**: 所有権の移動が不要な場合は参照を使用
4. **可変性**: 必要な場合のみ`mut`を使用
5. **エラーハンドリング**: 将来的にResult型を使用した適切なエラー処理を実装予定