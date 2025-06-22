# Yuni言語 型システム

Yuni言語は、コンパイル時の型安全性と実行時のパフォーマンスを両立させる強力な型システムを提供します。Rustの安全性、Goのシンプルさ、C++の表現力を組み合わせた現代的な型システムです。

## 型システムの基本原則

1. **静的型付け**: すべての型はコンパイル時に決定される
2. **型安全性**: 型の不一致によるバグをコンパイル時に検出
3. **型推論**: 多くの場面で型注釈を省略可能
4. **ゼロコスト抽象化**: 型システムによるオーバーヘッドなし
5. **メモリ安全性**: 型システムによるメモリ安全の保証

## 基本型

### 整数型

Yuniは幅広い整数型をサポートしています：

```yuni
// 符号付き整数
type SignedIntegers {
    i8   // -128 から 127
    i16  // -32,768 から 32,767
    i32  // -2,147,483,648 から 2,147,483,647 (デフォルト整数型)
    i64  // -9,223,372,036,854,775,808 から 9,223,372,036,854,775,807
    i128 // 128ビット符号付き整数
    i256 // 256ビット符号付き整数（大きな数値計算用）
}

// 符号無し整数
type UnsignedIntegers {
    u8   // 0 から 255
    u16  // 0 から 65,535
    u32  // 0 から 4,294,967,295
    u64  // 0 から 18,446,744,073,709,551,615
    u128 // 128ビット符号無し整数
    u256 // 256ビット符号無し整数（暗号計算用）
}
```

#### 整数リテラルと型推論

```yuni
let a = 42;           // i32として推論（デフォルト）
let b = 42i64;        // 明示的にi64
let c: u32 = 42;      // 型注釈でu32
let d = 0xFF;         // 16進数リテラル
let e = 0b1010;       // 2進数リテラル  
let f = 0o755;        // 8進数リテラル
let g = 1_000_000;    // アンダースコア区切り
```

### 浮動小数点数型

```yuni
type FloatTypes {
    f8   // 8ビット浮動小数点数（研究用）
    f16  // 16ビット浮動小数点数（Half precision）
    f32  // 32ビット浮動小数点数（Single precision）
    f64  // 64ビット浮動小数点数（Double precision、デフォルト）
}

let pi = 3.14159;       // f64として推論
let e = 2.718f32;       // f32として明示
let small: f16 = 1.0;   // f16として注釈
```

### 真偽値型

```yuni
type bool {
    true,
    false
}

let is_valid = true;
let is_ready: bool = false;
```

### 文字列型

```yuni
type String = [u8];  // UTF-8エンコードされたバイト配列

let message = "Hello, World!";
let empty = "";
let multiline = "Line 1\nLine 2";

// テンプレート文字列
let name = "Alice";
let greeting = `Hello, ${name}!`;
```

### void型（ユニット型）

```yuni
type void = ();

fn print_message(msg: String): void {
    println(msg);
    // return (); は暗黙的
}
```

## 複合型

### 配列型

```yuni
// 動的サイズ配列
type Array<T> = [T];

let numbers: [i32] = [1, 2, 3, 4, 5];
let strings: [String] = ["hello", "world"];
let empty: [f64] = [];

// 配列操作
let first = numbers[0];        // インデックスアクセス
let length = numbers.length;   // 長さ取得（将来実装予定）
```

### タプル型

```yuni
// 固定長の異種型コレクション
type Point2D = (f64, f64);
type Person = (String, i32, bool);

let origin: (f64, f64) = (0.0, 0.0);
let person: (String, i32, bool) = ("Alice", 25, true);

// タプル分解
let (x, y) = origin;
let (name, age, is_student) = person;
```

### 参照型

Yuniの参照システムはRustにインスパイアされていますが、より使いやすく設計されています：

```yuni
// 不変参照
type Ref<T> = &T;

// 可変参照  
type MutRef<T> = &mut T;

let value = 42;
let ref_to_value: &i32 = &value;      // 不変参照
let mut mutable_value = 42;
let mut_ref: &mut i32 = &mut mutable_value; // 可変参照

// 自動参照取得
fn takes_reference(x: &i32) {
    println("Value: ", x);
}

takes_reference(value);  // 自動的に&valueとして渡される
```

## ユーザー定義型

### 構造体型

```yuni
// 基本的な構造体
type Point struct {
    x: f64,
    y: f64
}

// フィールドの可視性（将来実装予定）
type Rectangle struct {
    pub width: f64,    // 公開フィールド
    pub height: f64,   // 公開フィールド
    area: f64          // プライベートフィールド
}

// ネストした構造体
type Circle struct {
    center: Point,
    radius: f64
}

// 構造体の初期化
let point = Point { x: 3.0, y: 4.0 };
let circle = Circle { 
    center: Point { x: 0.0, y: 0.0 }, 
    radius: 5.0 
};
```

### 代数的データ型（Enum）

```yuni
// 基本的なenum
type Color enum {
    Red,
    Green,
    Blue
}

// データを持つenum
type Option<T> enum {
    Some(T),
    None
}

type Result<T, E> enum {
    Ok(T),
    Err(E)
}

// 複雑なenum
type Shape enum {
    Circle(center: Point, radius: f64),
    Rectangle(top_left: Point, width: f64, height: f64),
    Triangle(a: Point, b: Point, c: Point)
}

// enumの使用
let color = Color::Red;
let maybe_value = Option::Some(42);
let result = Result::Ok("success");
let shape = Shape::Circle(Point { x: 0.0, y: 0.0 }, 5.0);
```

## 関数型

```yuni
// 関数ポインタ型
type FnPtr<Args, Return> = fn(Args) -> Return;

// 例
type BinaryOp = fn(i32, i32) -> i32;
type Predicate<T> = fn(T) -> bool;

fn add(a: i32, b: i32): i32 {
    return a + b;
}

let operation: BinaryOp = add;
let result = operation(10, 20);
```

## 型推論

Yuniの型推論エンジンは、可能な限り型注釈を不要にします：

```yuni
// 基本的な型推論
let x = 42;           // i32
let y = 3.14;         // f64
let z = true;         // bool
let s = "hello";      // String

// 複合型の推論
let point = Point { x: 1.0, y: 2.0 };  // Point
let numbers = [1, 2, 3];               // [i32]
let pair = (1, "hello");               // (i32, String)

// 関数からの型推論
fn get_number(): i32 {
    return 42;
}

let num = get_number();  // i32として推論

// コンテキストからの推論
let mut vector: [f64] = [];
vector.push(3.14);  // 3.14はf64として推論される（将来実装予定）
```

## 型変換とキャスト

### 明示的型変換

```yuni
let x: i32 = 42;
let y: f64 = x as f64;        // i32 → f64
let z: i64 = y as i64;        // f64 → i64（切り捨て）
let w: u32 = x as u32;        // i32 → u32

// 安全でない変換は警告される
let big: i64 = 9223372036854775807;
let small: i32 = big as i32;  // 警告: 値が切り詰められる可能性
```

### 暗黙的型変換

```yuni
// 数値型のプロモーション（小さい型から大きい型へ）
let a: i32 = 42;
let b: i64 = a;      // i32 → i64は自動変換
let c: f64 = a;      // i32 → f64は自動変換

// 参照の自動取得
fn takes_ref(x: &i32) {}
let value = 42;
takes_ref(value);    // 自動的に&valueとして変換
```

## 型システムの高度な機能

### ジェネリクス（将来実装予定）

```yuni
// ジェネリック関数
fn identity<T>(x: T): T {
    return x;
}

// ジェネリック構造体
type Box<T> struct {
    value: T
}

// 制約付きジェネリクス
fn add_numbers<T: Numeric>(a: T, b: T): T {
    return a + b;
}
```

### トレイト（将来実装予定）

```yuni
// トレイト定義
trait Display {
    fn display(self: &Self): String;
}

trait Clone {
    fn clone(self: &Self): Self;
}

// トレイト実装
impl Display for Point {
    fn display(self: &Self): String {
        return `(${self.x}, ${self.y})`;
    }
}
```

### 型エイリアス

```yuni
// 基本的な型エイリアス
type UserId = i64;
type UserName = String;
type Coordinates = (f64, f64);

// ジェネリック型エイリアス（将来実装予定）
type Result<T> = Result<T, String>;
type HashMap<K, V> = Map<K, V>;

let user_id: UserId = 12345;
let position: Coordinates = (10.5, 20.3);
```

## メモリレイアウトと型のサイズ

### 基本型のサイズ

```yuni
// 整数型のサイズ（バイト）
i8, u8     → 1 byte
i16, u16   → 2 bytes  
i32, u32   → 4 bytes
i64, u64   → 8 bytes
i128, u128 → 16 bytes
i256, u256 → 32 bytes

// 浮動小数点数型のサイズ
f8  → 1 byte
f16 → 2 bytes
f32 → 4 bytes  
f64 → 8 bytes

// その他
bool   → 1 byte
String → pointer (8 bytes on 64-bit)
&T     → 8 bytes (64-bitポインタ)
```

### 構造体のメモリレイアウト

```yuni
type Example struct {
    a: i32,    // 4 bytes
    b: i64,    // 8 bytes  
    c: bool    // 1 byte
}
// 合計: 16 bytes（パディングを含む）

// パディングの制御（将来実装予定）
#[packed]
type PackedStruct struct {
    a: i32,
    b: i64,
    c: bool
}
// 合計: 13 bytes（パディングなし）
```

## 型チェックとエラー処理

### コンパイル時型チェック

```yuni
let x: i32 = "hello";        // エラー: 型の不一致
let y: String = 42;          // エラー: i32をStringに代入不可

fn takes_string(s: String) {}
takes_string(42);            // エラー: 引数の型が不一致

let mut a: i32 = 42;
let b: &mut i32 = &a;        // エラー: aがmutでない
```

### 型安全な配列アクセス

```yuni
let numbers = [1, 2, 3];
let index: i32 = -1;
let value = numbers[index];  // 実行時境界チェック（将来実装予定）
```

### Null安全性

Yuniでは、nullポインタアクセスを型システムで防ぎます：

```yuni
// null値は存在しない - 代わりにOption型を使用
let maybe_value: Option<i32> = Option::Some(42);
let no_value: Option<i32> = Option::None;

// 安全なアクセス方法（将来実装予定）
match maybe_value {
    Some(value) => println("Value: ", value),
    None => println("No value")
}
```

## 型システムのベストプラクティス

### 1. 適切な型選択

```yuni
// ✅ 良い例
let age: u8 = 25;           // 年齢は0-255で十分
let count: u32 = 1000000;   // カウンタには適切なサイズ
let precise: f64 = 3.141592653589793; // 精密な計算

// ❌ 避けるべき例
let age: i64 = 25;          // オーバーサイズ
let count: i8 = 100;        // アンダーサイズのリスク
```

### 2. 参照の適切な使用

```yuni
// ✅ 良い例 - 読み取り専用
fn calculate_area(rectangle: &Rectangle): f64 {
    return rectangle.width * rectangle.height;
}

// ✅ 良い例 - 変更が必要
fn scale(rectangle: &mut Rectangle, factor: f64) {
    rectangle.width *= factor;
    rectangle.height *= factor;
}

// ❌ 避けるべき例 - 不必要な所有権移動
fn print_area(rectangle: Rectangle): f64 {
    return rectangle.width * rectangle.height;
    // rectangleはここで破棄される
}
```

### 3. 型注釈の使い分け

```yuni
// ✅ 型が明確な場合は省略
let count = 0;
let message = "hello";
let point = Point { x: 1.0, y: 2.0 };

// ✅ 型が不明確な場合は明示
let result: Result<i32, String> = parse_number(input);
let collection: [String] = [];
let callback: fn(i32) -> bool = |x| x > 0;
```

### 4. エラーハンドリングパターン

```yuni
// ✅ Result型での安全なエラーハンドリング（将来実装予定）
fn divide(a: f64, b: f64): Result<f64, String> {
    if b == 0.0 {
        return Result::Err("Division by zero");
    }
    return Result::Ok(a / b);
}

// ✅ Option型での値の存在チェック（将来実装予定）
fn find_user(id: UserId): Option<User> {
    // データベース検索ロジック
    if user_exists(id) {
        return Option::Some(get_user(id));
    } else {
        return Option::None;
    }
}
```

## まとめ

Yuni言語の型システムは以下の特徴を持ちます：

1. **強力な型安全性**: コンパイル時にほとんどのバグを検出
2. **直感的な型推論**: 最小限の型注釈で最大の安全性
3. **ゼロコスト抽象化**: 実行時オーバーヘッドなしの高レベル機能
4. **メモリ安全性**: 参照システムによる安全なメモリアクセス
5. **表現力**: 複雑なデータ構造を自然に表現

この型システムにより、Yuniは高性能でありながら安全で保守しやすいコードの記述を可能にします。