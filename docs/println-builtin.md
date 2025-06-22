# println組み込み関数

`println`関数は、コンソールに値を出力するYuni言語の組み込み関数です。

## 機能

- **可変長引数**: 任意の個数の引数を受け取ります（ゼロ個も含む）
- **自動文字列変換**: すべての引数型を自動的に文字列に変換します
- **スペース区切り**: 複数の引数はスペースで区切られます
- **型サポート**: 以下の型に対応:
  - 文字列リテラルと変数
  - 整数型（すべてのサイズ）
  - 浮動小数点型（すべてのサイズ）
  - ブール値（"true"または"false"として出力）

## 使用例

```yuni
// 空のprintlnは空行を出力
println();

// 単一の文字列引数
println("Hello, World!");

// 複数の引数
println("答えは", 42);

// 異なる型の変数
let name = "Alice";
let age = 25;
let height = 5.6;
let is_student = true;

println("名前:", name, "年齢:", age, "身長:", height, "学生:", is_student);
// 出力: 名前: Alice 年齢: 25 身長: 5.6 学生: true
```

## 実装詳細

### 意味解析
- `println`関数は意味解析器で組み込み関数として認識されます
- 可変長引数のため通常の関数型チェックをバイパスします
- すべての引数は有効な式であることが型チェックされます

### コード生成
- 各引数は型固有の変換関数を使用して文字列に変換されます:
  - 整数用の`yuni_i64_to_string`
  - 浮動小数点用の`yuni_f64_to_string`
  - ブール値用の`yuni_bool_to_string`
  - 文字列値は直接使用
- 引数は`yuni_string_concat`を使用してスペースで連結されます
- 最終的な文字列は`yuni_println`ランタイム関数に渡されます

### ランタイム要件
以下のランタイム関数が提供される必要があります:
- `yuni_println(str: *const u8)` - 文字列を出力し改行を追加
- `yuni_string_concat(a: *const u8, b: *const u8) -> *const u8` - 2つの文字列を連結
- `yuni_i64_to_string(val: i64) -> *const u8` - 整数を文字列に変換
- `yuni_f64_to_string(val: f64) -> *const u8` - 浮動小数点を文字列に変換
- `yuni_bool_to_string(val: bool) -> *const u8` - ブール値を文字列に変換