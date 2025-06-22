# Yuni Programming Language

Yuni言語は、パフォーマンスを犠牲にすることなくメモリ安全性を提供するように設計されたプログラミング言語です。RustとGoとC++の良いところを組み合わせて、簡単かつ統一された文法を持った表現力の豊かな言語です。

## 特徴

- **メモリ安全性**: Rust風の所有権システムでメモリ安全を保証
- **高パフォーマンス**: LLVM バックエンドによる最適化された実行ファイル生成
- **豊富な型システム**: 基本型、構造体、enum、代数的データ型をサポート
- **パターンマッチング**: 強力で表現力豊かなパターンマッチング機能
- **自動参照取得**: C++のような自動参照取得で使いやすさを向上
- **ムーブセマンティクス**: 効率的なメモリ管理

## クイックスタート

### Hello, World!

```yuni
package main

fn main() {
    println("Hello, World!");
}
```

### 基本的な算術演算

```yuni
package main

fn add(a: i32, b: i32): i32 {
    return a + b;
}

fn main() {
    let x: i32 = 10i32;
    let y: i32 = 20i32;
    let result: i32 = add(x, y);
    println("結果:", result);
}
```

## ドキュメント

詳細な言語仕様とガイドについては、以下のドキュメントをご参照ください：

### 言語仕様
- **[構文仕様](docs/syntax.md)** - Yuni言語の完全な構文ガイド
- **[型システム](docs/type-system.md)** - 型システムの詳細仕様
- **[所有権システム](docs/ownership.md)** - メモリ安全性を保証する所有権システム
- **[パターンマッチング](docs/pattern-match.md)** - 強力なパターンマッチング機能の詳細

### 標準ライブラリとツール
- **[標準ライブラリ](docs/standard-library.md)** - 組み込み関数と標準ライブラリの仕様
- **[コンパイラ使用方法](docs/compiler-usage.md)** - Yuniコンパイラの詳細な使用ガイド

### 追加資料
- **[言語概要](docs/language-overview.md)** - Yuni言語の基本的な文法と機能（概要）
- **[println ビルトイン関数](docs/println-builtin.md)** - 組み込み出力関数の技術仕様

## プロジェクト構造

Yuni言語のRust実装コンパイラです。LLVMを使用してコード生成を行います。

### ディレクトリ構造

```
yunilang/
├── src/
│   ├── main.rs          # コンパイラエントリーポイントとCLI
│   ├── lexer/           # トークン化と字句解析
│   ├── parser/          # 構文解析とAST構築
│   ├── ast/             # 抽象構文木の定義
│   ├── analyzer/        # 意味解析と型チェック
│   ├── codegen/         # LLVM コード生成
│   └── runtime/         # ランタイム サポート関数
├── examples/            # サンプル Yuni プログラム
├── docs/                # 言語ドキュメント
├── tests/               # テストファイル
├── build.rs             # LLVM ビルド設定
└── Cargo.toml           # Rust プロジェクト設定
```

## 必要環境

### LLVM 18

YuniコンパイラはLLVM 18がシステムにインストールされている必要があります。

#### macOS (Homebrew使用)
```bash
brew install llvm@18
export LLVM_SYS_180_PREFIX=$(brew --prefix llvm@18)
```

#### Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install llvm-18-dev
```

#### その他のシステム
[llvm.org](https://llvm.org/)からLLVM 18をダウンロード・インストールし、以下の環境変数を設定してください：
```bash
export LLVM_SYS_180_PREFIX=/path/to/llvm-18
```

### Rust

[rustup.rs](https://rustup.rs/)からRustをインストール：
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## ビルド

LLVMがインストールされ設定が完了したら：

```bash
cargo build
```

最適化されたリリースビルドの場合：
```bash
cargo build --release
```

## 使用方法

Yuniコンパイラは以下のコマンドを提供します：

### ファイルのコンパイル
```bash
# オブジェクトファイルにコンパイル
cargo run -- compile input.yuni

# LLVM IRにコンパイル
cargo run -- compile input.yuni --emit-llvm

# カスタム出力でコンパイル
cargo run -- compile input.yuni -o output.o
```

### ファイルの実行 (JITコンパイル)
```bash
cargo run -- run input.yuni
```

### コンパイルせずに構文チェック
```bash
cargo run -- check input.yuni
```

### インタラクティブREPLの開始
```bash
cargo run -- repl
```

### 追加オプション
- `--verbose` または `-v`: 詳細出力を有効にする
- `--dump-ast`: ASTを出力する
- `--dump-tokens`: レキサーからのトークンを出力する
- `-O<level>`: 最適化レベルを設定 (0-3)

## 開発

### テストの実行
```bash
cargo test
```

### ドキュメントのビルド
```bash
cargo doc --open
```

### コードフォーマット
```bash
cargo fmt
```

### リンティング
```bash
cargo clippy
```

## 依存関係

- **inkwell**: RustのためのLLVMセーフバインディング
- **llvm-sys**: 低レベルLLVMバインディング
- **clap**: コマンドライン引数パース
- **logos**: 高速レキサージェネレーター
- **nom**: パーサーコンビネータライブラリ
- **thiserror/anyhow**: エラーハンドリング
- **codespan-reporting**: 美しいエラー診断
- **serde**: ASTダンプのためのシリアライゼーション

