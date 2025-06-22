# Yuni言語 所有権システム

Yuni言語の所有権システムは、Rustにインスパイアされたメモリ安全性メカニズムでありながら、より直感的で使いやすく設計されています。ガベージコレクションを使用せずに、コンパイル時にメモリ安全性を保証します。

## 所有権システムの基本原則

1. **一意所有権**: 各値は一度に一つの所有者のみを持つ
2. **自動メモリ管理**: スコープを抜ける際に自動的にメモリが解放される
3. **借用**: 所有権を移動せずに値にアクセスする仕組み
4. **ライフタイム**: 参照の有効期間をコンパイル時に保証
5. **自動参照取得**: 多くの場面で明示的な参照操作を不要にする

## 所有権の基本概念

### 値の所有

```yuni
fn main() {
    let x = 42;           // xが値42を所有
    let y = x;            // 基本型はコピーされる（xは依然として有効）
    
    let s1 = "Hello";     // s1が文字列を所有
    let s2 = s1;          // 所有権がs1からs2に移動（s1は無効になる）
    
    // println(s1);       // エラー: s1はもう使用できない
    println(s2);          // OK: s2が所有権を持っている
}
```

### 基本型 vs 複合型

```yuni
// 基本型（Copy trait を実装）- 常にコピーされる
type CopyTypes {
    i8, i16, i32, i64, i128, i256,
    u8, u16, u32, u64, u128, u256,
    f8, f16, f32, f64,
    bool,
    char
}

// 複合型 - 所有権が移動する
type MoveTypes {
    String,
    [T],        // 配列
    Struct {},  // 構造体
    Enum {}     // 代数的データ型
}

fn example() {
    // コピー型
    let a = 42;
    let b = a;      // aをコピー、aは依然として有効
    println(a, b);  // OK
    
    // ムーブ型
    let s1 = "Hello";
    let s2 = s1;    // s1からs2に所有権移動
    // println(s1); // エラー: s1は無効
    println(s2);    // OK
}
```

## 借用システム（Borrowing）

### 不変借用（Immutable Borrowing）

```yuni
fn print_length(s: &String) {
    println("Length: ", s.length);  // 読み取り専用アクセス
}

fn main() {
    let message = "Hello, World!";
    print_length(&message);  // 不変借用を渡す
    println(message);        // messageは依然として使用可能
}
```

### 可変借用（Mutable Borrowing）

```yuni
fn append_text(s: &mut String, text: &String) {
    s.push_str(text);  // 文字列を変更
}

fn main() {
    let mut message = "Hello";
    append_text(&mut message, ", World!");
    println(message);  // "Hello, World!"
}
```

### 借用規則

```yuni
fn borrowing_rules() {
    let mut data = "Hello";
    
    // 規則1: 複数の不変借用は同時に存在可能
    let ref1 = &data;
    let ref2 = &data;
    let ref3 = &data;
    println(ref1, ref2, ref3);  // OK
    
    // 規則2: 可変借用は一つだけ
    let mut_ref = &mut data;
    // let another_mut = &mut data;  // エラー: 同時に複数の可変借用は不可
    
    // 規則3: 可変借用と不変借用は同時に存在不可
    // let read_ref = &data;  // エラー: 可変借用と同時は不可
    
    println(mut_ref);
}
```

## 自動参照取得

Yuniの特徴的な機能として、多くの場面で自動的に参照が取得されます：

```yuni
fn takes_reference(x: &i32) {
    println("Value: ", x);
}

fn takes_mut_reference(x: &mut String) {
    x.push_str(" World");
}

fn main() {
    let value = 42;
    takes_reference(value);  // 自動的に&valueとして渡される
    
    let mut text = "Hello";
    takes_mut_reference(text);  // 自動的に&mut textとして渡される
    
    // 明示的な参照取得も可能
    takes_reference(&value);
    takes_mut_reference(&mut text);
}
```

### 左辺への自動参照取得

```yuni
fn main() {
    let value = 42;
    let reference: &i32 = value;  // 自動的に&valueとして参照を取得
    
    let mut mutable = 100;
    let mut_ref: &mut i32 = mutable;  // 自動的に&mut mutableとして取得
}
```

## ライフタイムシステム

### 基本的なライフタイム

```yuni
fn main() {
    let r;                    // ライフタイム 'a 開始
    {
        let x = 5;           // ライフタイム 'b 開始
        r = &x;              // エラー: xのライフタイムがrより短い
    }                        // ライフタイム 'b 終了
    println(r);              // エラー: rが無効な参照を指している
}                            // ライフタイム 'a 終了
```

### 関数のライフタイム注釈

```yuni
// 戻り値のライフタイムを明示的に指定
fn longest(x: &String, y: &String): &String {
    if x.length > y.length {
        return x;
    } else {
        return y;
    }
}

// より複雑なライフタイム注釈
fn create_reference(data: &String): (result: &String)
lives
    result = data
{
    return data;
}
```

### 構造体のライフタイム

```yuni
type Borrower struct {
    name: &String,        // 借用した文字列への参照
    value: &i32          // 借用した整数への参照
}

fn main() {
    let text = "Alice";
    let number = 42;
    
    let borrower = Borrower {
        name: &text,      // textを借用
        value: &number    // numberを借用
    };
    
    // borrowerが存在する限り、textとnumberは有効でなければならない
    println(borrower.name, borrower.value);
}
```

## メモリ管理パターン

### RAII（Resource Acquisition Is Initialization）

```yuni
type File struct {
    handle: i32,
    name: String
}

fn (f: File) open(filename: String): File {
    // ファイルを開く
    return File { 
        handle: system_open(filename), 
        name: filename 
    };
}

fn (f: &mut File) write(data: String) {
    system_write(f.handle, data);
}

fn (f: File) close() {
    system_close(f.handle);  // デストラクタで自動的に呼ばれる
}

fn main() {
    let file = File::open("data.txt");
    file.write("Hello, World!");
    // ここでfileがスコープを抜け、自動的にcloseが呼ばれる
}
```

### 所有権の移動と返却

```yuni
fn process_data(data: String): String {
    // データを処理
    let processed = data + " (processed)";
    return processed;  // 所有権を返す
}

fn main() {
    let original = "Hello";
    let result = process_data(original);  // originalの所有権を移動
    // println(original);  // エラー: originalは無効
    println(result);       // OK: resultが所有権を持つ
}
```

## 高度な所有権パターン

### 所有権の共有（将来実装予定）

```yuni
// 参照カウント（Rc）
type Rc<T> struct {
    data: T,
    count: u32
}

fn (rc: Rc<T>) clone(): Rc<T> {
    // 参照カウントを増加
    rc.count += 1;
    return Rc { data: rc.data, count: rc.count };
}

// 使用例
fn main() {
    let shared_data = Rc::new("Shared text");
    let reference1 = shared_data.clone();
    let reference2 = shared_data.clone();
    
    // すべての参照が有効
    println(shared_data.data);
    println(reference1.data);
    println(reference2.data);
}
```

### 内部可変性（将来実装予定）

```yuni
// Cell型による内部可変性
type Cell<T> struct {
    value: T
}

fn (cell: &Cell<T>) get(): T {
    return cell.value;
}

fn (cell: &Cell<T>) set(value: T) {
    cell.value = value;  // 不変参照経由でも変更可能
}

fn main() {
    let data = Cell::new(42);
    let ref_to_data = &data;  // 不変参照
    
    println(ref_to_data.get());  // 42
    ref_to_data.set(100);        // 不変参照経由で変更
    println(ref_to_data.get());  // 100
}
```

## エラーパターンと解決策

### よくある所有権エラー

```yuni
// エラー1: 使用後の移動
fn error_example1() {
    let s = "Hello";
    let s2 = s;      // 所有権移動
    println(s);      // エラー: sは無効
    
    // 解決策: 借用を使用
    let s = "Hello";
    let s2 = &s;     // 借用
    println(s);      // OK: sは依然として有効
}

// エラー2: 複数の可変借用
fn error_example2() {
    let mut data = "Hello";
    let ref1 = &mut data;
    let ref2 = &mut data;  // エラー: 複数の可変借用
    
    // 解決策: 借用のスコープを分ける
    let mut data = "Hello";
    {
        let ref1 = &mut data;
        // ref1を使用
    }
    let ref2 = &mut data;  // OK: ref1はスコープ外
}

// エラー3: ダングリングポインタ
fn error_example3(): &String {
    let local = "Hello";
    return &local;   // エラー: ローカル変数への参照を返却
    
    // 解決策: 所有権を移動
    fn fixed_example3(): String {
        let local = "Hello";
        return local;  // OK: 所有権を移動
    }
}
```

## 所有権のベストプラクティス

### 1. 適切な所有権戦略の選択

```yuni
// ✅ 読み取り専用の場合は借用
fn calculate_length(s: &String): i32 {
    return s.length;
}

// ✅ 変更が必要な場合は可変借用
fn make_uppercase(s: &mut String) {
    // 文字列を大文字に変換
}

// ✅ 所有権が必要な場合は移動
fn process_and_store(s: String): ProcessedData {
    // データを処理して保存
    return ProcessedData { content: s };
}
```

### 2. 効率的なデータ構造設計

```yuni
// ✅ 借用を活用した効率的な構造体
type DocumentProcessor struct {
    config: &Config,      // 設定への参照
    cache: &mut Cache     // キャッシュへの可変参照
}

// ❌ 不必要な所有権を持つ構造体
type IneffientProcessor struct {
    config: Config,       // 設定のコピー（無駄）
    cache: Cache          // キャッシュのコピー（無駄）
}
```

### 3. ライフタイムの明確化

```yuni
// ✅ 明確なライフタイム関係
fn get_first_word(text: &String): &String
lives
    return = text
{
    // 最初の単語を抽出して返す
    // 戻り値のライフタイムがtextに依存することが明確
}

// ✅ 複雑な場合の適切な注釈
fn merge_data(a: &Data, b: &Data): (result: &Data)
lives
    result = a  // または result = b
{
    if a.priority > b.priority {
        return a;
    } else {
        return b;
    }
}
```

### 4. パフォーマンスを考慮した設計

```yuni
// ✅ 大きなデータは借用で渡す
fn process_large_data(data: &LargeStruct) {
    // データを処理（コピーのオーバーヘッドなし）
}

// ✅ 小さなデータはコピーで渡す
fn calculate(x: i32, y: i32): i32 {
    return x + y;  // 基本型はコピーが効率的
}

// ❌ 小さなデータの不必要な借用
fn inefficient_add(x: &i32, y: &i32): i32 {
    return *x + *y;  // 参照外しのオーバーヘッド
}
```

## スマートポインタ（将来実装予定）

### Box: ヒープ割り当て

```yuni
type Box<T> struct {
    ptr: *T
}

fn main() {
    let boxed_value = Box::new(42);  // ヒープに割り当て
    println(*boxed_value);           // 42
    // boxedValueがスコープを抜ける際にヒープメモリが自動解放
}
```

### Arc: アトミック参照カウント

```yuni
type Arc<T> struct {
    data: T,
    count: AtomicU32
}

fn share_data_between_threads() {
    let shared = Arc::new("Shared data");
    let handle1 = Arc::clone(&shared);
    let handle2 = Arc::clone(&shared);
    
    // 複数のスレッドで安全に共有可能
    thread::spawn(move || {
        println(handle1);
    });
    
    thread::spawn(move || {
        println(handle2);
    });
}
```

## まとめ

Yuni言語の所有権システムは以下の利点を提供します：

1. **メモリ安全性**: コンパイル時にメモリリークやダングリングポインタを防止
2. **ゼロコスト抽象化**: ランタイムオーバーヘッドなしの安全性
3. **直感的な設計**: 自動参照取得により使いやすさを向上
4. **予測可能性**: 明確な所有権ルールによる予測可能な動作
5. **高パフォーマンス**: ガベージコレクションなしの効率的なメモリ管理

この所有権システムにより、Yuniは安全性とパフォーマンスを両立した現代的なシステムプログラミング言語として設計されています。