# Yunilangのパターンマッチング

パターンマッチングは、データを分解し、パターンに対して簡潔で表現力豊かな方法でマッチングを行うことができる強力な機能です。

## 基本構文

基本的なパターンマッチング構文は `match` キーワードを使用します：

```yuni
match 式 {
    パターン1 => 式1,
    パターン2 => 式2,
    ...
    _ => デフォルト式
}
```

## パターンの種類

### リテラルパターン

リテラル値に対してマッチします：

```yuni
match x {
    0 => "ゼロ",
    1 => "いち",
    2 => "に",
    _ => "その他"
}
```

### 変数パターン

値を変数に束縛します：

```yuni
match value {
    x => x * 2  // xは任意の値に束縛される
}
```

### ワイルドカードパターン

アンダースコア `_` は束縛せずに任意の値にマッチします：

```yuni
match value {
    0 => "ゼロ",
    _ => "ゼロ以外"
}
```

### タプルパターン

タプルを分解します：

```yuni
match point {
    (0, 0) => "原点",
    (x, 0) => "x軸上",
    (0, y) => "y軸上",
    (x, y) => "点({x}, {y})"
}
```

### リストパターン

様々なパターンでリストにマッチします：

```yuni
match list {
    [] => "空",
    [x] => "単一要素: {x}",
    [x, y] => "2つの要素",
    [head, ...tail] => "先頭: {head}, 残り: {tail}",
    [1, 2, ...rest] => "1, 2で始まる",
    _ => "その他"
}
```

### レコードパターン

レコード/構造体を分解します：

```yuni
match person {
    { name: "アリス", age } => "アリスは{age}歳です",
    { name, age: 30 } => "{name}は30歳",
    { name, age, ...rest } => "人物 {name}、年齢 {age}",
    _ => "不明"
}
```

### 型パターン

型に対してマッチします：

```yuni
match value {
    n: i32 => "整数: {n}",
    s: String => "文字列: {s}",
    f: Float => "浮動小数点: {f}",
    _ => "その他の型"
}
```

### Enum/バリアントパターン

代数的データ型に対してマッチします：

```yuni
type Option<T> enum {
    Some(T),
    None
}

match option {
    Some(value) => value,
    None => default_value
}

type Result<T, E> enum {
    Ok(T),
    Err(E)
}

match result {
    Ok(data) => process(data),
    Err(error) => handle_error(error)
}
```


## ガード節

`if` を使用してパターンに条件を追加します：

```yuni
match x {
    n if n > 0 => "正の数",
    n if n < 0 => "負の数",
    _ => "ゼロ"
}
```

## ネストされたパターン

複雑なマッチングのためにパターンをネストできます：

```yuni
match data {
    Some((x, y)) if x > y => "x > yのペア",
    Some((x, x)) => "同じ値のペア",
    Some((x, y)) => "ペア({x}, {y})",
    None => "なし"
}
```

## Orパターン

`|` で複数のパターンにマッチします：

```yuni
match value {
    0 | 1 | 2 => "小",
    3 | 4 | 5 => "中",
    _ => "大"
}
```

## 範囲パターン

範囲に対してマッチします。`...` は終端を含まず、`..=` は終端を含みます：

```yuni
match age {
    0...18 => "未成年",       // 0〜17
    18...65 => "成人",        // 18〜64
    65... => "高齢者",        // 65以上
    _ => "無効"
}

match score {
    0..=59 => "不可",         // 0〜59（59を含む）
    60..=69 => "可",          // 60〜69（69を含む）
    70..=79 => "良",          // 70〜79（79を含む）
    80..=89 => "優",          // 80〜89（89を含む）
    90..=100 => "秀",         // 90〜100（100を含む）
    _ => "無効なスコア"
}
```

## 関数内でのパターンマッチング

関数内でmatch式を使用してパターンマッチングを行います：

```yuni
fn factorial(n: i32): i32 {
    match n {
        0 => 1,
        n => n * factorial(n - 1)
    }
}

fn map<a, b>(f: a -> b, list: List<a>): List<b> {
    match list {
        [] => [],
        [head, ...tail] => [f(head), ...map(f, tail)]
    }
}

// より複雑な例
fn process<a>(list: List<a>): String {
    match list {
        [] => "空のリスト",
        [x] => "単一要素: {x}",
        [x, y] => "2つの要素: {x}, {y}",
        [x, y, z, ...rest] => "3つ以上の要素"
    }
}

// ガード節も使用可能
fn abs(n: i32): i32 {
    match n {
        n if n >= 0 => n,
        n => -n
    }
}

// 複数の引数をタプルでマッチング
fn compare(a: i32, b: i32): String {
    match (a, b) {
        (x, y) if x > y => "greater",
        (x, y) if x < y => "less",
        _ => "equal"
    }
}
```

## let束縛でのパターンマッチング

let束縛で値を分解します：

```yuni
let (x, y) = point;
let { name, age } = person;
let [first, second, ...rest] = list;
```

## 網羅性チェック

コンパイラはすべての可能なパターンがカバーされていることを保証します：

```yuni
// Colorにより多くのバリアントがある場合、コンパイルエラーになる
match color {
    Red => "#FF0000",
    Green => "#00FF00",
    Blue => "#0000FF"
    // より多くの色が存在する場合、パターン不足エラー
}
```

## 例

### ツリー走査

```yuni
// 再帰型は自動的にBox化される
type Tree<T> enum {
    Leaf(T),
    Node(Tree<T>, T, Tree<T>)  // 内部的にはBox<Tree<T>>として扱われる
}

// シンプルな構文でツリーを操作
fn (t: &Tree<i32>) Sum(): i32 {
    match t {
        Leaf(n) => n,
        Node(left, value, right) => left.Sum() + value + right.Sum()
    }
}

// ツリーの構築も自然な構文で
fn create_sample_tree(): Tree<i32> {
    Node(
        Leaf(1),
        2,
        Node(
            Leaf(3),
            4,
            Leaf(5)
        )
    )
}

// LinkedListの例
type LinkedList<T> enum {
    Nil,
    Cons(T, LinkedList<T>)  // 自動的にBox化される
}
```

### 再帰型の自動メモリ管理

Yunilangでは、再帰型は自動的に適切なメモリ配置が行われます：

1. **自動ヒープ割り当て**: enum内で自己参照する型は自動的にヒープに配置される
2. **透過的な構文**: プログラマはメモリ配置を意識する必要がない
3. **メモリ安全性**: コンパイラが適切なメモリ管理を保証

```yuni
// 再帰型の定義例
type BinaryTree<T> enum {
    Empty,
    Node(BinaryTree<T>, T, BinaryTree<T>)  // 自動的に適切なメモリ配置
}

// 自然な構文で使用
let tree = Node(
    Node(Empty, 1, Empty),
    2,
    Node(Empty, 3, Empty)
)
```

### Option処理

```yuni
fn safe_divide(x, y) {
    match y {
        0 => None,
        _ => Some(x / y)
    }
}

fn process_result(opt) {
    match opt {
        Some(value) => "結果: {value}",
        None => "ゼロ除算"
    }
}
```

### ステートマシン

```yuni
type State enum {
    Initial,
    Processing(i32),
    Complete(String),
    Error(String)
}

fn transition(state, event) {
    match (state, event) {
        (Initial, Start) => Processing(0),
        (Processing(n), Increment) => Processing(n + 1),
        (Processing(n), Finish) => Complete("{n}個のアイテムを処理"),
        (_, Error(msg)) => Error(msg),
        (s, _) => s  // 現在の状態を維持
    }
}
```

## ベストプラクティス

1. **特定から一般の順序でパターンを並べる**: より特定的なパターンを一般的なパターンの前に配置
2. **ワイルドカードは控えめに使用**: エラー検出のため明示的なパターンを優先
3. **網羅性チェックを活用**: すべてのケースを処理するためコンパイラに助けてもらう
4. **複雑な条件にはガードを使用**: パターンはシンプルに保ち、追加ロジックにはガードを使用
5. **深く分解する**: 複数のmatch式を避けるためネストパターンを活用

## パターンマッチング vs If-Else

以下の場合、if-elseチェーンよりもパターンマッチングが推奨されます：
- 値だけでなく構造に対してマッチする場合
- 複数の関連する条件を処理する場合
- 代数的データ型を扱う場合
- コンパイル時の網羅性チェックが必要な場合

```yuni
// パターンマッチングを推奨
match option {
    Some(x) if x > 0 => positive_action(x),
    Some(x) => negative_action(x),
    None => default_action()
}

// if-elseチェーンよりも優れている
if is_some(option) && get_value(option) > 0 {
    positive_action(get_value(option))
} else if is_some(option) {
    negative_action(get_value(option))
} else {
    default_action()
}
```