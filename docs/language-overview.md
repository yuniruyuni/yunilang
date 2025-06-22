# 言語概要

Yuni言語は、パフォーマンスを犠牲にすることなくメモリ安全性を提供するように設計されたプログラミング言語です。
RustとGoとC++の良いところを組み合わせて、簡単かつ統一された文法を持った表現力の豊かな言語です。

## サンプル

### Hello, World!

```yuni
package main

fn main() {
    println("Hello, World!");
}
```

### 基本型

```yuni
package main

fn main() {
    // rustのような各種基本型が存在する
    // 整数: i8, i16, i32, u64, u128, u256
    // 非負整数: u8, u16, u32, u64, u128, u256
    // 浮動小数点数: f8, f16, f32, f64
    let a: i8 = 100; // 定数は左辺に代入するときに型が推論される
    let b: i8 = 200i8; // Rustのように定数の型を明示的に指示することもできる。

    println("a: ", a);
    println("b: ", b);
}
```

### 複合型

```yuni
package main

// 構造体
type Point struct {
    x: f64,
    y: f64,
}

// 代数的データ型
type State enum {
    Init,
    Running(rest: i32),
    Finished,
}

fn main() {
    let p = Point { x: 3.0, y: 5.5 };
    let s = State::Running(10);
}
```

### 書き換え可能変数

```yuni
package main

type Point struct {
    x: f64,
    y: f64,
}

fn main() {
    let p: Point = Point{ x: 3.0, y: 5.0 };

    p.x = 100.0; // Error! pは書き換え不能変数。C++のconst型のような振る舞い
    
    let p: mut Point = Point { x: 3.0, y: 5.0 };
    p.x = 100.0; // OK. 書き換え可能。
}
```

### 関数

```yuni
package main

// Rustと異なり、 `str` はなく String sliceは `&String` で表現する。
fn greet(name: &String): &String {
    // 変数展開のできる文字列は `` によって作ることができる。
    return `Hello, ${name}!`;
}

fn main()  {
    let message = greet("Yuniru");
    println(message);
}
```

### メソッド

```yuni
package main

// go言語風のimport構文。常にimport対象を()で囲むことを求める
import (
    "math"
)

type Point struct {
    x: f32;
    y: f32;
}

// ノルムを求める
fn (p: &Point) Norm(): f32 {
    (p.x * p.x) + (p.y * p.y)
}

// 長さを求める
fn (p: &Point) Length(): f32 {
    math.sqrt(p.Norm())
} 

// 正規化する
fn (p: &mut Point) Normalize() {
    let len = p.Length();
    if len <=  f32::epsilon {
        return
    }

    p.x /= len;
    p.y /= len;
}

fn main()  {
    let mut p = Point{ x: 12.0, y: 16.0 };

    println("x: ", p.x, ", y: ", p.y);
    println("Length: ", p.Length());

    p.Normalize();

    println("x: ", p.x, ", y: ", p.y);
    println("Length: ", p.Length());
}
```

### メソッド

```yuni
package main

// Rustと異なり、 `str` はなく String sliceは `&String` で表現する。
fn greet(name: &String): &String {
    // 変数展開のできる文字列は `` によって作ることができる。
    return `Hello, ${name}!`;
}

fn main()  {
    let message = greet("Yuniru");
    println(message);
}
```


### 参照と自動参照取得

```yuni
package main

fn show(val: &i32) {
    println("value: ", val);
}

fn main() {
    let value = 128;
    show(value); // Rustと異なり、引数の参照は自動的に取得される。

    let ref: &i32 = value; // 左辺の型が参照の場合、自動的に参照が取得される。
    let ref2: &i32 = ref; // 右辺が参照で左辺も参照の場合は単純に代入となる。
}
```

### ムーブセマンティクス

```yuni
package main

type Messenger struct {
    message: &String;
}

fn (Messenger) new(message: &String): (ret: Messenger)
lives
    ret = message,
{
    // この戻り値はmain側のpにmoveされる
    Messenger{ message }
}

fn (m: &Messenger) Say() {
    println(m.message);
}

fn main() {
    let s = "Hello!";
    let m = Messenger::new(s);
}
```
