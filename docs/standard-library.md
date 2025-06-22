# Yuni言語 標準ライブラリ

Yuni言語の標準ライブラリは、効率的で安全なプログラミングに必要な基本的な機能を提供します。モジュラー設計により、必要な機能のみをインポートして使用できます。

## 現在実装済みの機能

### 組み込み関数

#### println

文字列と値をコンソールに出力する基本的な出力関数です。

```yuni
// 基本的な使用法
println();                    // 空行を出力
println("Hello, World!");     // 文字列を出力
println("Answer:", 42);       // 複数の引数をスペース区切りで出力

// 変数の出力
let name = "Alice";
let age = 25;
println("Name:", name, "Age:", age);
// 出力: Name: Alice Age: 25

// 様々な型の出力
println("Integer:", 42);       // 整数
println("Float:", 3.14);       // 浮動小数点数
println("Boolean:", true);     // 真偽値
println("String:", "text");    // 文字列
```

**技術仕様:**
- 可変長引数（0個以上の引数を受け取り可能）
- 自動型変換（すべての基本型を文字列に変換）
- スペース区切りでの複数引数出力
- 改行文字の自動付加

## 計画中の標準ライブラリモジュール

### core（コアモジュール）

基本的なデータ型と操作を提供します。

```yuni
// 基本型の拡張メソッド（計画中）
import (
    "core/option"
    "core/result"
    "core/iter"
)

// Option型による安全なnull値処理
type Option<T> enum {
    Some(T),
    None
}

fn example_option() {
    let maybe_value: Option<i32> = Option::Some(42);
    
    match maybe_value {
        Some(val) => println("Value:", val),
        None => println("No value")
    }
}

// Result型による安全なエラーハンドリング
type Result<T, E> enum {
    Ok(T),
    Err(E)
}

fn divide(a: f64, b: f64): Result<f64, String> {
    if b == 0.0 {
        return Result::Err("Division by zero");
    }
    return Result::Ok(a / b);
}
```

### collections（コレクション）

データ構造と操作を提供します。

```yuni
import (
    "collections/vector"
    "collections/hashmap"
    "collections/set"
)

// 動的配列
type Vector<T> struct {
    data: [T],
    capacity: u64,
    length: u64
}

fn vector_example() {
    let mut vec = vector::Vector::new();
    vec.push(1);
    vec.push(2);
    vec.push(3);
    
    for item in vec {
        println(item);
    }
}

// ハッシュマップ
type HashMap<K, V> struct {
    buckets: [Bucket<K, V>],
    size: u64
}

fn hashmap_example() {
    let mut map = hashmap::HashMap::new();
    map.insert("key1", "value1");
    map.insert("key2", "value2");
    
    match map.get("key1") {
        Some(value) => println("Found:", value),
        None => println("Not found")
    }
}
```

### string（文字列処理）

文字列操作のための包括的な機能を提供します。

```yuni
import (
    "string"
)

fn string_example() {
    let text = "Hello, World!";
    
    // 文字列検索
    if string::contains(text, "World") {
        println("Found 'World'");
    }
    
    // 文字列分割
    let parts = string::split(text, ", ");
    for part in parts {
        println("Part:", part);
    }
    
    // 文字列変換
    let upper = string::to_upper(text);
    let lower = string::to_lower(text);
    
    // 文字列フォーマット
    let formatted = string::format("Hello, {}! You are {} years old.", "Alice", 25);
}
```

### math（数学関数）

数学計算のための関数群を提供します。

```yuni
import (
    "math"
)

fn math_example() {
    let x = 3.14159;
    
    // 三角関数
    let sin_x = math::sin(x);
    let cos_x = math::cos(x);
    let tan_x = math::tan(x);
    
    // 指数・対数関数
    let exp_x = math::exp(x);
    let log_x = math::log(x);
    let log10_x = math::log10(x);
    
    // 平方根・累乗
    let sqrt_x = math::sqrt(x);
    let pow_x = math::pow(x, 2.0);
    
    // 定数
    let pi = math::PI;
    let e = math::E;
}
```

### io（入出力）

ファイルシステムやネットワークI/Oを提供します。

```yuni
import (
    "io/file"
    "io/path"
)

fn file_example() {
    // ファイル読み取り
    match file::read_to_string("data.txt") {
        Ok(content) => println("File content:", content),
        Err(error) => println("Error reading file:", error)
    }
    
    // ファイル書き込み
    let data = "Hello, File!";
    match file::write("output.txt", data) {
        Ok(_) => println("File written successfully"),
        Err(error) => println("Error writing file:", error)
    }
    
    // パス操作
    let full_path = path::join("/home/user", "documents/file.txt");
    let filename = path::filename(full_path);
    let extension = path::extension(full_path);
}
```

### sync（同期プリミティブ）

並行プログラミングのための同期メカニズムを提供します。

```yuni
import (
    "sync/mutex"
    "sync/channel"
    "sync/atomic"
)

fn concurrency_example() {
    // ミューテックス
    let mut data = mutex::Mutex::new(0);
    {
        let mut guard = data.lock();
        *guard += 1;
    } // ロックはここで自動的に解放される
    
    // チャンネル
    let (sender, receiver) = channel::channel();
    
    thread::spawn(move || {
        sender.send("Hello from thread");
    });
    
    match receiver.recv() {
        Ok(msg) => println("Received:", msg),
        Err(_) => println("Channel closed")
    }
}
```

### net（ネットワーク）

ネットワーク通信のための機能を提供します。

```yuni
import (
    "net/tcp"
    "net/http"
)

fn network_example() {
    // TCPサーバー
    let listener = tcp::listen("127.0.0.1:8080");
    for connection in listener.incoming() {
        match connection {
            Ok(stream) => handle_client(stream),
            Err(error) => println("Connection error:", error)
        }
    }
    
    // HTTPクライアント
    match http::get("https://api.example.com/data") {
        Ok(response) => {
            println("Status:", response.status);
            println("Body:", response.body);
        },
        Err(error) => println("HTTP error:", error)
    }
}
```

### time（時間処理）

時間と日付の処理機能を提供します。

```yuni
import (
    "time"
)

fn time_example() {
    // 現在時刻
    let now = time::now();
    println("Current time:", now);
    
    // 時間の計算
    let tomorrow = now + time::Duration::days(1);
    let hour_ago = now - time::Duration::hours(1);
    
    // 時間のフォーマット
    let formatted = time::format(now, "%Y-%m-%d %H:%M:%S");
    println("Formatted:", formatted);
    
    // タイマー
    time::sleep(time::Duration::seconds(1));
}
```

### thread（スレッド処理）

マルチスレッドプログラミングのサポートを提供します。

```yuni
import (
    "thread"
)

fn threading_example() {
    // 新しいスレッドを生成
    let handle = thread::spawn(|| {
        for i in 1..=5 {
            println("Thread:", i);
            thread::sleep(time::Duration::milliseconds(100));
        }
    });
    
    // メインスレッドの処理
    for i in 1..=3 {
        println("Main:", i);
        thread::sleep(time::Duration::milliseconds(150));
    }
    
    // スレッドの完了を待機
    handle.join();
}
```

### crypto（暗号化）

暗号化とハッシュ機能を提供します。

```yuni
import (
    "crypto/hash"
    "crypto/aes"
    "crypto/rsa"
)

fn crypto_example() {
    let data = "Hello, Crypto!";
    
    // ハッシュ計算
    let sha256 = hash::sha256(data);
    let md5 = hash::md5(data);
    
    // 対称暗号化
    let key = aes::generate_key();
    let encrypted = aes::encrypt(data, key);
    let decrypted = aes::decrypt(encrypted, key);
    
    // 非対称暗号化
    let (public_key, private_key) = rsa::generate_keypair();
    let encrypted_rsa = rsa::encrypt(data, public_key);
    let decrypted_rsa = rsa::decrypt(encrypted_rsa, private_key);
}
```

### json（JSON処理）

JSON形式のデータ処理機能を提供します。

```yuni
import (
    "json"
)

type Person struct {
    name: String,
    age: i32,
    email: String
}

fn json_example() {
    let person = Person {
        name: "Alice",
        age: 30,
        email: "alice@example.com"
    };
    
    // シリアライゼーション
    let json_string = json::serialize(person);
    println("JSON:", json_string);
    
    // デシリアライゼーション
    match json::deserialize<Person>(json_string) {
        Ok(parsed_person) => println("Parsed:", parsed_person.name),
        Err(error) => println("Parse error:", error)
    }
}
```

## 型変換ユーティリティ

### 基本型変換

```yuni
// 現在実装済み（println内部で使用）
fn conversion_example() {
    let num = 42;
    let float_num = 3.14;
    let flag = true;
    
    // 文字列変換（組み込み関数として利用可能）
    // 内部実装：
    // yuni_i64_to_string(num) -> "42"
    // yuni_f64_to_string(float_num) -> "3.14"  
    // yuni_bool_to_string(flag) -> "true"
    
    println(num, float_num, flag);
}
```

### 将来的な型変換API

```yuni
import (
    "convert"
)

fn advanced_conversion() {
    // 文字列からの変換
    let parsed_int = convert::parse_int("42");        // Result<i32, ParseError>
    let parsed_float = convert::parse_float("3.14");  // Result<f64, ParseError>
    let parsed_bool = convert::parse_bool("true");    // Result<bool, ParseError>
    
    // バイト配列変換
    let bytes = convert::to_bytes("Hello");           // [u8]
    let string = convert::from_bytes(bytes);          // Result<String, Utf8Error>
}
```

## エラーハンドリングパターン

### 標準エラー型

```yuni
// 計画中の標準エラー型
type Error enum {
    IoError(String),
    ParseError(String),
    NetworkError(String),
    TimeoutError,
    OutOfMemoryError
}

// エラーの使用例
fn error_handling_example() {
    match risky_operation() {
        Ok(result) => println("Success:", result),
        Err(Error::IoError(msg)) => println("IO Error:", msg),
        Err(Error::ParseError(msg)) => println("Parse Error:", msg),
        Err(error) => println("Other error:", error)
    }
}
```

## メモリ管理ユーティリティ

### スマートポインタ（計画中）

```yuni
import (
    "memory/box"
    "memory/rc"
    "memory/arc"
)

fn memory_example() {
    // ヒープ割り当て
    let boxed = box::Box::new(42);
    println(*boxed);
    
    // 参照カウント
    let shared = rc::Rc::new("Shared data");
    let reference = rc::Rc::clone(&shared);
    
    // アトミック参照カウント（スレッド安全）
    let thread_safe = arc::Arc::new("Thread safe data");
    let thread_ref = arc::Arc::clone(&thread_safe);
}
```

## テスト支援

### テストフレームワーク（計画中）

```yuni
import (
    "test"
)

#[test]
fn test_addition() {
    let result = add(2, 3);
    test::assert_eq(result, 5);
}

#[test]
fn test_division() {
    let result = divide(10.0, 2.0);
    match result {
        Ok(value) => test::assert_eq(value, 5.0),
        Err(_) => test::fail("Unexpected error")
    }
}
```

## 標準ライブラリの使用方法

### インポート構文

```yuni
// 単一モジュールのインポート
import (
    "math"
)

// 複数モジュールのインポート
import (
    "string"
    "collections/vector"
    "io/file"
)

// エイリアスの使用
import (
    "collections/vector" as vec
    "collections/hashmap" as map
)

// 特定の関数のインポート（計画中）
import (
    "math" { sin, cos, sqrt }
)
```

### モジュール使用例

```yuni
package main

import (
    "math"
    "string"
    "collections/vector"
)

fn main() {
    // 数学計算
    let angle = math::PI / 4.0;
    let sin_value = math::sin(angle);
    
    // 文字列操作
    let text = "Hello, World!";
    let upper_text = string::to_upper(text);
    
    // コレクション操作
    let mut numbers = vector::Vector::new();
    numbers.push(1);
    numbers.push(2);
    numbers.push(3);
    
    for num in numbers {
        println("Number:", num);
    }
}
```

## 実装ロードマップ

### 短期目標（現在開発中）
- [ ] Option/Result型の実装
- [ ] 基本的なString操作
- [ ] Vector（動的配列）の実装
- [ ] 基本的なmath関数

### 中期目標
- [ ] ファイルI/O操作
- [ ] HashMap実装
- [ ] 基本的な並行処理
- [ ] JSON処理

### 長期目標
- [ ] ネットワーク機能
- [ ] 暗号化ライブラリ
- [ ] 高度な並行処理
- [ ] テストフレームワーク

## パフォーマンス特性

Yuni標準ライブラリは以下の設計原則に従います：

1. **ゼロコスト抽象化**: 抽象化によるランタイムオーバーヘッドなし
2. **メモリ効率**: 最小限のメモリ使用量
3. **型安全性**: コンパイル時エラー検出
4. **予測可能性**: 一貫した動作とパフォーマンス
5. **相互運用性**: C/C++との容易な連携

この標準ライブラリにより、Yuni言語は実用的なシステムプログラミング言語として必要な機能を包括的に提供します。