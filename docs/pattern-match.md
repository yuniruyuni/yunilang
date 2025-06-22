# Yunilangのパターンマッチング

パターンマッチングは、データを分解し、パターンに対して簡潔で表現力豊かな方法でマッチングを行うことができる強力な機能です。

## 基本構文

基本的なパターンマッチング構文は `match` キーワードを使用します：

```yunilang
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

```yunilang
match x {
    0 => "ゼロ",
    1 => "いち",
    2 => "に",
    _ => "その他"
}
```

### 変数パターン

値を変数に束縛します：

```yunilang
match value {
    x => x * 2  // xは任意の値に束縛される
}
```

### ワイルドカードパターン

アンダースコア `_` は束縛せずに任意の値にマッチします：

```yunilang
match value {
    0 => "ゼロ",
    _ => "ゼロ以外"
}
```

### タプルパターン

タプルを分解します：

```yunilang
match point {
    (0, 0) => "原点",
    (x, 0) => "x軸上",
    (0, y) => "y軸上",
    (x, y) => "点({x}, {y})"
}
```

### リストパターン

様々なパターンでリストにマッチします：

```yunilang
match list {
    [] => "空",
    [x] => "単一要素: {x}",
    [x, y] => "2つの要素",
    [head | tail] => "先頭: {head}, 残り: {tail}",
    [1, 2, ..rest] => "1, 2で始まる",
    _ => "その他"
}
```

### レコードパターン

レコード/構造体を分解します：

```yunilang
match person {
    { name: "アリス", age } => "アリスは{age}歳です",
    { name, age: 30 } => "{name}は30歳",
    { name, age, ..rest } => "人物 {name}、年齢 {age}",
    _ => "不明"
}
```

### 型パターン

型に対してマッチします：

```yunilang
match value {
    n: Int => "整数: {n}",
    s: String => "文字列: {s}",
    f: Float => "浮動小数点: {f}",
    _ => "その他の型"
}
```

### Enum/バリアントパターン

代数的データ型に対してマッチします：

```yunilang
enum Option<T> {
    Some(T),
    None
}

match option {
    Some(value) => value,
    None => default_value
}

enum Result<T, E> {
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

```yunilang
match x {
    n if n > 0 => "正の数",
    n if n < 0 => "負の数",
    _ => "ゼロ"
}
```

## ネストされたパターン

複雑なマッチングのためにパターンをネストできます：

```yunilang
match data {
    Some((x, y)) if x > y => "x > yのペア",
    Some((x, x)) => "同じ値のペア",
    Some((x, y)) => "ペア({x}, {y})",
    None => "なし"
}
```

## Orパターン

`|` で複数のパターンにマッチします：

```yunilang
match value {
    0 | 1 | 2 => "小",
    3 | 4 | 5 => "中",
    _ => "大"
}
```

## 範囲パターン

範囲に対してマッチします：

```yunilang
match age {
    0..18 => "未成年",
    18..65 => "成人",
    65.. => "高齢者",
    _ => "無効"
}
```

## 関数引数でのパターンマッチング

関数はパラメータで直接パターンマッチできます：

```yunilang
fn factorial {
    0 => 1,
    n => n * factorial(n - 1)
}

fn map {
    (_, []) => [],
    (f, [head | tail]) => [f(head) | map(f, tail)]
}
```

## let束縛でのパターンマッチング

let束縛で値を分解します：

```yunilang
let (x, y) = point;
let { name, age } = person;
let [first, second | rest] = list;
```

## 網羅性チェック

コンパイラはすべての可能なパターンがカバーされていることを保証します：

```yunilang
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

```yunilang
enum Tree<T> {
    Leaf(T),
    Node(Tree<T>, T, Tree<T>)
}

fn sum_tree {
    Leaf(n) => n,
    Node(left, value, right) => sum_tree(left) + value + sum_tree(right)
}
```

### Option処理

```yunilang
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

```yunilang
enum State {
    Initial,
    Processing(Int),
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

```yunilang
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