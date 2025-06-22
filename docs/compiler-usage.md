# Yuniコンパイラ 使用方法

Yuniコンパイラは、Yuni言語のソースコードを効率的な実行ファイルにコンパイルするためのコマンドラインツールです。LLVM 18をバックエンドとして使用し、高度な最適化と柔軟な出力オプションを提供します。

## コマンドライン構文

```bash
cargo run -- <サブコマンド> [オプション] [ファイル]
```

または、コンパイル済みバイナリを使用：

```bash
./yuni <サブコマンド> [オプション] [ファイル]
```

## サブコマンド

### compile - ファイルのコンパイル

Yuniソースファイルを実行ファイルまたは指定された形式にコンパイルします。

```bash
# 基本的なコンパイル（実行ファイルを生成）
cargo run -- compile program.yuni

# 出力ファイル名を指定
cargo run -- compile program.yuni -o my_program

# 最適化レベルを指定
cargo run -- compile program.yuni -O3

# LLVM IRを出力
cargo run -- compile program.yuni --emit llvm-ir

# オブジェクトファイルを出力
cargo run -- compile program.yuni --emit obj

# アセンブリコードを出力
cargo run -- compile program.yuni --emit asm

# 詳細なコンパイル情報を表示
cargo run -- compile program.yuni --verbose
```

#### compileオプション

| オプション | 短縮形 | 説明 | デフォルト |
|-----------|--------|------|------------|
| `--output` | `-o` | 出力ファイル名を指定 | 入力ファイル名から推測 |
| `--optimize` | `-O` | 最適化レベル (0-3) | 0 |
| `--emit` | | 出力形式を指定 | executable |
| `--verbose` | `-v` | 詳細な情報を表示 | false |
| `--dump-ast` | | ASTをJSON形式で出力 | false |
| `--dump-tokens` | | トークンリストを出力 | false |

#### 出力形式（--emit）

- `executable`: 実行ファイル（デフォルト）
- `llvm-ir`: LLVM中間表現（.ll）
- `obj`: オブジェクトファイル（.o）
- `asm`: アセンブリコード（.s）

### run - ファイルの実行

JITコンパイルを使用してYuniプログラムを直接実行します。

```bash
# プログラムを直接実行
cargo run -- run hello.yuni

# 詳細情報を表示しながら実行
cargo run -- run hello.yuni --verbose

# 最適化して実行
cargo run -- run hello.yuni -O2
```

#### runオプション

| オプション | 短縮形 | 説明 | デフォルト |
|-----------|--------|------|------------|
| `--optimize` | `-O` | 最適化レベル (0-3) | 0 |
| `--verbose` | `-v` | 詳細な情報を表示 | false |

### check - 構文チェック

コンパイルせずに構文とセマンティクスのチェックのみを実行します。

```bash
# 構文チェック
cargo run -- check program.yuni

# 詳細なエラー情報を表示
cargo run -- check program.yuni --verbose

# ASTダンプも実行
cargo run -- check program.yuni --dump-ast
```

#### checkオプション

| オプション | 短縮形 | 説明 | デフォルト |
|-----------|--------|------|------------|
| `--verbose` | `-v` | 詳細な情報を表示 | false |
| `--dump-ast` | | ASTをJSON形式で出力 | false |
| `--dump-tokens` | | トークンリストを出力 | false |

### repl - インタラクティブモード

対話的にYuniコードを実行できるREPL（Read-Eval-Print Loop）を開始します。

```bash
# REPLを開始
cargo run -- repl

# 詳細モードでREPL開始
cargo run -- repl --verbose
```

#### replオプション

| オプション | 短縮形 | 説明 | デフォルト |
|-----------|--------|------|------------|
| `--verbose` | `-v` | 詳細な情報を表示 | false |

#### REPL使用方法

```bash
$ cargo run -- repl
Yuni REPL v0.1.0
Type 'exit' to quit, 'help' for help

yuni> let x = 42;
yuni> println("Hello, ", x);
Hello, 42
yuni> fn add(a: i32, b: i32): i32 { return a + b; }
yuni> add(10, 20)
30
yuni> exit
```

## コンパイル例

### 基本的なHello Worldプログラム

```bash
# ファイル: hello.yuni
package main

fn main() {
    println("Hello, World!");
}
```

```bash
# コンパイルして実行
$ cargo run -- compile hello.yuni
$ ./hello
Hello, World!

# 直接実行
$ cargo run -- run hello.yuni
Hello, World!
```

### 最適化されたコンパイル

```bash
# リリースビルド（最大最適化）
$ cargo run -- compile program.yuni -O3 -o program_optimized

# サイズ最適化（計画中）
$ cargo run -- compile program.yuni -Os -o program_small

# デバッグ情報付きコンパイル（計画中）
$ cargo run -- compile program.yuni -g -o program_debug
```

### 開発者向けオプション

```bash
# AST構造を確認
$ cargo run -- check program.yuni --dump-ast

# トークン解析結果を確認
$ cargo run -- check program.yuni --dump-tokens

# LLVM IRを出力して確認
$ cargo run -- compile program.yuni --emit llvm-ir
$ cat program.ll

# 詳細なコンパイル過程を表示
$ cargo run -- compile program.yuni --verbose
```

## エラーメッセージ

Yuniコンパイラは、codespan-reportingライブラリを使用して美しく読みやすいエラーメッセージを提供します。

### 構文エラーの例

```bash
$ cargo run -- check invalid.yuni
error: 期待されないトークンです
  ┌─ invalid.yuni:3:5
  │
3 │     let x = ;
  │         ^ ここで式が期待されていますが、';' が見つかりました
  │
  = ヒント: 変数宣言では値を指定する必要があります

error: コンパイルに失敗しました
```

### 型エラーの例

```bash
$ cargo run -- check type_error.yuni
error: 型の不一致
  ┌─ type_error.yuni:4:13
  │
4 │     let x: i32 = "hello";
  │            ---   ^^^^^^^ String型の値が渡されましたが
  │            │
  │            i32型が期待されています
  │
  = ヒント: 文字列を数値に変換する場合は parse() 関数を使用してください

error: コンパイルに失敗しました
```

## 環境変数

Yuniコンパイラの動作を制御するための環境変数：

| 環境変数 | 説明 | デフォルト |
|----------|------|------------|
| `LLVM_SYS_180_PREFIX` | LLVM 18のインストールパス | 自動検出 |
| `YUNI_DEBUG` | デバッグ情報の表示レベル | 0 |
| `YUNI_TARGET` | ターゲットアーキテクチャ | ホストアーキテクチャ |

### 使用例

```bash
# LLVM パスを手動指定
$ LLVM_SYS_180_PREFIX=/usr/local/llvm-18 cargo run -- compile program.yuni

# デバッグモードでコンパイル
$ YUNI_DEBUG=1 cargo run -- compile program.yuni --verbose

# クロスコンパイル（計画中）
$ YUNI_TARGET=aarch64-linux-gnu cargo run -- compile program.yuni
```

## パフォーマンス最適化

### 最適化レベル

| レベル | オプション | 説明 | 使用場面 |
|--------|------------|------|----------|
| 0 | `-O0` | 最適化なし（デフォルト） | 開発・デバッグ |
| 1 | `-O1` | 基本的な最適化 | 開発・テスト |
| 2 | `-O2` | 標準的な最適化 | 本番環境 |
| 3 | `-O3` | 積極的な最適化 | 高性能が必要な場合 |

### 最適化の効果

```bash
# ベンチマーク用プログラム
$ cargo run -- compile benchmark.yuni -O0 -o bench_debug
$ cargo run -- compile benchmark.yuni -O3 -o bench_release

# 実行時間の比較
$ time ./bench_debug
real    0m2.451s

$ time ./bench_release  
real    0m0.123s
```

## トラブルシューティング

### よくある問題と解決方法

#### 1. LLVM が見つからない

```bash
error: LLVM 18 が見つかりません
```

**解決方法:**
```bash
# macOS (Homebrew)
$ brew install llvm@18
$ export LLVM_SYS_180_PREFIX=$(brew --prefix llvm@18)

# Ubuntu/Debian
$ sudo apt-get install llvm-18-dev
```

#### 2. リンクエラー

```bash
error: リンクに失敗しました: 未定義のシンボル 'yuni_println'
```

**解決方法:**
```bash
# ランタイムライブラリが正しくビルドされているか確認
$ cargo clean
$ cargo build
```

#### 3. メモリ不足

```bash
error: コンパイル中にメモリが不足しました
```

**解決方法:**
```bash
# 最適化レベルを下げる
$ cargo run -- compile large_program.yuni -O1

# または、より多くのメモリを確保する環境で実行
```

### デバッグ支援

#### 詳細ログの有効化

```bash
# 全ステージの詳細ログ
$ RUST_LOG=debug cargo run -- compile program.yuni --verbose

# 特定のモジュールのログのみ
$ RUST_LOG=yuni::parser=debug cargo run -- compile program.yuni
```

#### コンパイル中間結果の確認

```bash
# 字句解析結果
$ cargo run -- check program.yuni --dump-tokens

# 構文解析結果（AST）
$ cargo run -- check program.yuni --dump-ast

# LLVM IR
$ cargo run -- compile program.yuni --emit llvm-ir
$ cat program.ll
```

## CI/CD での使用

### GitHub Actions での設定例

```yaml
name: Yuni Build

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Install LLVM
      run: |
        sudo apt-get update
        sudo apt-get install llvm-18-dev
    
    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    
    - name: Build Yuni Compiler
      run: cargo build --release
    
    - name: Test Compilation
      run: |
        cargo run -- check examples/*.yuni
        cargo run -- compile examples/hello.yuni
        ./hello
```

### Makefileの例

```makefile
# Yuni プロジェクト用 Makefile

.PHONY: build test clean check

YUNI_COMPILER = cargo run --

build:
	cargo build --release

test: build
	$(YUNI_COMPILER) check examples/*.yuni
	$(YUNI_COMPILER) compile examples/hello.yuni
	./hello
	rm -f hello

check:
	$(YUNI_COMPILER) check src/*.yuni

clean:
	cargo clean
	rm -f *.o *.ll *.s hello

install:
	cargo install --path .

.DEFAULT_GOAL := build
```
