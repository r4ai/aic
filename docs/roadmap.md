# 言語開発ロードマップ

以下のフェーズでイテレーションを回しながら、Inkwell を利用して LLVM IR を経由し、最終的に実行可能ファイルを生成する実験的コンパイル言語を開発します。

## フェーズ 1: 仕様策定・設計 (1–2 週間)

- 言語の目的とターゲット（LLVM IR 生成、実験的）を明確化
- Rust 風＋ Go っぽい文法のコア構文を定義
  - 式（`if`/`while`/`for`）、関数、モジュール宣言の文法例
- AST（抽象構文木）のデータ構造設計
- LLVM IR へのマッピング（関数呼び出し、ブロック、ループの対応）

**成果物**

- BNF/EBNF で書いた簡易文法仕様書
- Rust の `enum Ast { … }` 設計スケッチ

## フェーズ 2: レキサー (logos)・パーサー (Chumsky) の導入 (2 週間)

- `logos` でトークナイザを書く
  - 識別子、キーワード、リテラル、記号トークン定義
- `chumsky` でパーサー骨格を実装
  - 文単位・式単位のパーサールール
  - `if`/関数呼び出し/ブロックの構文解析

**テスト**

- 各構文のユニットテスト
- 誤文法時のエラー位置・メッセージ確認

## フェーズ 3: AST 構築と型・名前解決 (2 週間)

- パーサー出力を AST に変換
- シンボルテーブル／スコープ管理
  - 変数・関数定義の重複チェック
- 型チェック（基本型のみ／動的型付けでも可）
- エラーハンドリング改善（位置情報付き）

**成果物**

- 名前解決と型チェックのパイプライン
- テストコードでの合格例・失敗例

## フェーズ 4: LLVM IR コード生成 (Inkwell) (3–4 週間)

- Inkwell ライブラリを使用して LLVM IR を生成するモジュール
- AST → LLVM IR へのマッピング設計
- 関数・ローカル変数・制御構造の emit 実装
- 標準ライブラリ（簡易入出力）の LLVM IR スケルトン

**検証**

- 生成された LLVM IR の妥当性を `llc` や `lli` で確認

## フェーズ 5: 実行可能ファイル生成・ビルド CLI (2-3 週間)

- 生成された LLVM IR をオブジェクトファイル (`.o`) にコンパイルする処理 (例: `llc` コマンドの呼び出し)
- オブジェクトファイルと必要なライブラリをリンクして実行可能ファイルを生成する処理 (例: `clang` や `lld` の呼び出し)
- 複数ファイル／モジュール間依存の解決とビルドプロセス統合
- `cargo run -- mylang build src/main.ml -o output_executable` のような CLI
- `--emit=llvm-ir` のような中間表現出力オプションの追加

**検証**

- コンパイルされた実行可能ファイルが正しく動作することを確認

## フェーズ 6: テスト・ドキュメント整備 (2 週間)

- 言語リファレンス／チュートリアルの Markdown ドキュメント
- CI（GitHub Actions）での自動ビルド＋テスト
- ベンチマーク（簡易）でパフォーマンス把握

## フェーズ 7: イテレーション・拡張 (継続)

- ジェネリクス、クロージャ、エラー回復強化などの実験的機能追加
- 最適化パス（LLVM 最適化パスの活用）
- クロスコンパイル対応の検討
