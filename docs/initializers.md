# 初期化子（Initializers）

Yuni言語における初期化子は、データ構造を簡潔に構築するための構文です。用途に応じて異なる形式の初期化子を使い分けることで、コードの意図を明確に表現できます。

## 初期化子の種類

### 1. リスト初期化子（List Initializer）

配列やベクターなど、同じ型の要素を順序付きで格納するコレクションの初期化に使用します。

```yuni
// 基本的な使用法
let vec = Vec<i32> [1, 2, 3, 4, 5];
let array = Array<f64> [1.0, 2.0, 3.0];

// 空のリスト
let empty_vec = Vec<String> [];

// ネストしたリスト
let matrix = Vec<Vec<i32>> [
    [1, 2, 3],
    [4, 5, 6],
    [7, 8, 9]
];
```

### 2. マップ初期化子（Map Initializer）

キーと値のペアを持つ辞書型データ構造の初期化に使用します。

```yuni
// 基本的な使用法
let scores = HashMap<String, i32> {
    "Alice": 95,
    "Bob": 87,
    "Charlie": 92
};

// 異なる型のキー
let config = HashMap<String, Any> {
    "host": "localhost",
    "port": 8080,
    "debug": true
};
```

### 3. 構造体初期化子（Struct Initializer）

構造体のフィールドを名前付きで初期化します。

```yuni
type Point struct {
    x: f64,
    y: f64
}

// 基本的な使用法
let point = Point { x: 10.0, y: 20.0 };

// フィールドの順序は自由
let point2 = Point { y: 30.0, x: 15.0 };
```

### 4. Enum初期化子（Enum Initializer）

代数的データ型（Enum）のバリアントを初期化します。単純なバリアントは関数呼び出し形式で、フィールド付きバリアントは構造体形式で初期化します。

```yuni
// 単純なバリアント（関数呼び出し形式）
let opt = Some(42);
let none = None();
let result = Ok(100);
let error = Err("error message");

// フィールド付きバリアント（構造体形式）
type Message enum {
    Text { content: String },
    Image { url: String, width: i32, height: i32 }
}

let msg = Message::Text { content: "Hello" };
let img = Message::Image { 
    url: "example.jpg", 
    width: 800, 
    height: 600 
};
```

## 暗黙的な初期化子変換

関数の引数が初期化可能な型である場合、初期化子を直接渡すことができます。コンパイラが自動的に適切な型のオブジェクトを構築します。

### 基本的な使用例

```yuni
fn process_point(p: Point) {
    println("Point: ({}, {})", p.x, p.y);
}

// 明示的な型名なしで初期化子を渡す
process_point({ x: 10.0, y: 20.0 });

fn calculate_sum(vec: Vec<i32>) -> i32 {
    vec.iter().sum()
}

// リスト初期化子を直接渡す
let total = calculate_sum([1, 2, 3, 4, 5]);
```

### 参照型への暗黙的変換

参照を受け取る関数にも初期化子を渡すことができます。一時オブジェクトが作成され、その参照が関数に渡されます。

```yuni
fn print_vec(v: &Vec<i32>) {
    for item in v {
        println(item);
    }
}

// 初期化子から一時オブジェクトを作成
print_vec([10, 20, 30]);

fn modify_map(m: &mut HashMap<String, i32>) {
    m.insert("new_key", 999);
}

// 可変参照への変換も可能
modify_map({ "initial": 100 });
```

### ネストした初期化子

複数の引数で初期化子を使用する場合：

```yuni
type Player struct {
    name: String,
    score: i32
}

fn create_game(player1: Player, player2: Player) {
    // ゲームロジック
}

// 両方の引数で初期化子を使用
create_game(
    { name: "Alice", score: 0 },
    { name: "Bob", score: 0 }
);
```

### メソッド呼び出しでの使用

メソッドの引数でも初期化子を使用できます：

```yuni
type Rectangle struct {
    width: f64,
    height: f64
}

impl Rectangle {
    fn contains(&self, point: Point) -> bool {
        // 判定ロジック
    }
    
    fn overlaps(&self, other: Rectangle) -> bool {
        // 重なり判定
    }
}

let rect = Rectangle { width: 100.0, height: 50.0 };

// メソッド引数での初期化子
if rect.contains({ x: 25.0, y: 25.0 }) {
    println("Point is inside");
}

if rect.overlaps({ width: 50.0, height: 30.0 }) {
    println("Rectangles overlap");
}
```

## 型の曖昧性と解決

暗黙的な初期化子変換で型が曖昧になる場合、コンパイラはエラーを報告します：

```yuni
type Point2D struct { x: f64, y: f64 }
type Size struct { x: f64, y: f64 }  // 同じフィールド名

fn process(p: Point2D) { }
fn process(s: Size) { }  // オーバーロード

// エラー：どちらの型か判断できない
process({ x: 10.0, y: 20.0 });  // コンパイルエラー

// 解決策：明示的に型を指定
process(Point2D { x: 10.0, y: 20.0 });
```

## ベストプラクティス

1. **型が明確な場合は暗黙的変換を活用**：コードが簡潔になり、可読性が向上します
2. **複雑な初期化は明示的に**：深くネストした構造や、多くのフィールドを持つ構造体は型名を明示
3. **一貫性を保つ**：プロジェクト内で初期化スタイルを統一

## 将来の拡張（検討中）

### カスタムコンストラクタでの初期化子パラメータ

初期化子を関数パラメータとして受け取る機能：

```yuni
// 初期化子型の定義（将来の機能）
fn make_validated_point(init: {x: f64, y: f64}) -> Result<Point, String> {
    if init.x < 0.0 || init.y < 0.0 {
        return Err("Negative coordinates not allowed");
    }
    Ok(Point { x: init.x, y: init.y })
}

// 使用例
let point = make_validated_point({ x: 10.0, y: 20.0 })?;
```

この機能により、初期化時のバリデーションやカスタムロジックを簡潔に記述できるようになります。