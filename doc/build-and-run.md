# BigQuery ADBCドライバーのビルドとRustからの実行手順

この文書では、dbt Labs版のArrow ADBCに含まれるGo製BigQueryドライバーを
macOS向けの動的ライブラリとしてビルドし、Rustプログラムからロードして
BigQueryへクエリを実行するまでの手順を説明します。

## 前提環境

- macOS（Apple Silicon）
- Git
- RustおよびCargo
- mise
- Google Cloud CLI（`gcloud`）
- BigQuery APIを利用できるGoogle Cloudプロジェクト

作業するリポジトリは次の場所を想定しています。

```text
/Users/nosu/work/git/ghq/github.com/nosu23/dbt-adbc-bigquery-rust-lab
```

Goドライバーのソースは、次のGit submoduleにあります。

```text
driver/arrow-adbc
```

## 1. submoduleを初期化する

新しくリポジトリをcloneした場合は、最初にsubmoduleを取得します。

```sh
cd /Users/nosu/work/git/ghq/github.com/nosu23/dbt-adbc-bigquery-rust-lab

git submodule update --init --recursive
```

現在のsubmoduleのコミットは次のコマンドで確認できます。

```sh
git submodule status
```

## 2. Goツールチェーンを準備する

このリポジトリでは、`mise.toml` にGoのバージョンを固定しています。

```sh
mise install
```

使用されるGoを確認します。

```sh
mise current go
which go
go version
```

想定するGoのバージョンは次のとおりです。

```text
go version go1.27.2 darwin/arm64
```

miseがシェルに反映されていない場合は、`~/.zshrc` に次の設定が必要です。

```sh
eval "$(mise activate zsh)"
```

設定後、ログインシェルを再起動します。

```sh
exec zsh -l
```

## 3. Go製BigQueryドライバーをビルドする

リポジトリのルートから次のコマンドを実行します。

```sh
cd /Users/nosu/work/git/ghq/github.com/nosu23/dbt-adbc-bigquery-rust-lab

make \
  -C driver/arrow-adbc/go/adbc/pkg \
  -B \
  BUILD_TAGS=driverlib,netgo,http2legacy \
  libadbc_driver_bigquery.dylib
```

各指定の意味は次のとおりです。

- `-C driver/arrow-adbc/go/adbc/pkg`: submodule内のドライバービルド用ディレクトリへ移動する
- `-B`: 既存の成果物があっても強制的に再ビルドする
- `driverlib`: ADBCドライバー共有ライブラリとしてビルドする
- `netgo`: GoのDNSリゾルバーを使用する
- `http2legacy`: Go 1.27と現在のgRPC依存関係を組み合わせてビルドするため、従来のHTTP/2実装を使用する

成功すると、次のファイルが生成されます。

```text
driver/arrow-adbc/go/adbc/pkg/libadbc_driver_bigquery.dylib
```

`.dylib` はmacOSの動的ライブラリです。単独で実行するファイルではなく、
RustのADBC Driver Managerから実行時にロードされます。

## 4. 生成されたドライバーを確認する

```sh
ls -lh driver/arrow-adbc/go/adbc/pkg/libadbc_driver_bigquery.dylib

file driver/arrow-adbc/go/adbc/pkg/libadbc_driver_bigquery.dylib
```

Apple Siliconでは、`file` の結果に次の内容が含まれます。

```text
Mach-O 64-bit dynamically linked shared library arm64
```

ADBC初期化関数が公開されていることは次のコマンドで確認できます。

```sh
nm -gU driver/arrow-adbc/go/adbc/pkg/libadbc_driver_bigquery.dylib \
  | rg 'AdbcDriver.*Init'
```

## 5. Rustプログラムをビルドする

Rustプログラムは、submodule内の次のcrateをパス依存として使用します。

```text
driver/arrow-adbc/rust/core
driver/arrow-adbc/rust/driver_manager
```

まずコンパイル確認を行います。

```sh
cargo check -p dbt-adbc-bigquery-rust-lab
```

実行ファイルをビルドする場合は次を実行します。

```sh
cargo build -p dbt-adbc-bigquery-rust-lab
```

デバッグ用実行ファイルは次の場所に生成されます。

```text
target/debug/dbt-adbc-bigquery-rust-lab
```

submodule内の既存コードに由来する非推奨API警告が表示される場合がありますが、
`Finished` と表示されればビルドは成功しています。

## 6. Application Default Credentialsを準備する

Rustプログラムは、Go製BigQueryドライバーを通じてGoogle Cloudの
Application Default Credentials（ADC）を使用します。

```sh
gcloud auth application-default login
```

必要に応じて、ADCのquota projectを設定します。

```sh
gcloud auth application-default set-quota-project PROJECT_ID
```

`PROJECT_ID` は利用するGoogle CloudプロジェクトIDに置き換えてください。

## 7. BigQueryへクエリを実行する

既定のクエリは次のSQLです。

```sql
SELECT 42 AS answer
```

プロジェクトIDを指定して実行します。

```sh
cargo run -p dbt-adbc-bigquery-rust-lab -- \
  --project PROJECT_ID
```

成功すると、結果がArrowのテーブルとして標準出力へ表示されます。

任意のSQLを実行する場合は、`--query` を指定します。

```sh
cargo run -p dbt-adbc-bigquery-rust-lab -- \
  --project PROJECT_ID \
  --query 'SELECT CURRENT_TIMESTAMP() AS current_time'
```

`--project` の代わりに環境変数も使用できます。

```sh
export GOOGLE_CLOUD_PROJECT=PROJECT_ID
cargo run -p dbt-adbc-bigquery-rust-lab
```

別の場所にあるドライバーをロードする場合は、`--driver` を指定します。

```sh
cargo run -p dbt-adbc-bigquery-rust-lab -- \
  --project PROJECT_ID \
  --driver /absolute/path/to/libadbc_driver_bigquery.dylib
```

CLIオプションは次のコマンドで確認できます。

```sh
cargo run -p dbt-adbc-bigquery-rust-lab -- --help
```

## トラブルシューティング

### `make: *** go/adbc/pkg: No such file or directory`

親リポジトリのルートから `-C go/adbc/pkg` を指定すると、パスが不足します。
次のパスを使用してください。

```sh
-C driver/arrow-adbc/go/adbc/pkg
```

### `undefined: errors.AsType`

古いGoツールチェーンが使用されています。次のコマンドでmise版のGoが
選択されているか確認してください。

```sh
mise current go
which go
go version
```

### `undefined: http2.TrailerPrefix`

Go 1.27と現在のgRPCおよび`x/net/http2`の組み合わせによるものです。
ビルド時に次の指定を含めてください。

```sh
BUILD_TAGS=driverlib,netgo,http2legacy
```

### ドライバーが見つからない

既定では次のファイルをロードします。

```text
driver/arrow-adbc/go/adbc/pkg/libadbc_driver_bigquery.dylib
```

ファイルが存在するか確認し、存在しない場合はGoドライバーを再ビルドしてください。

### 認証エラーになる

ADCを作成し直し、対象プロジェクトへアクセスできるGoogleアカウントで
ログインしていることを確認してください。

```sh
gcloud auth application-default login
```

## Gitで管理するもの

次のファイルはGitで管理します。

- Rustソースコード
- `Cargo.toml`
- `Cargo.lock`
- `mise.toml`
- この文書
- submoduleが参照するコミット

次のビルド成果物は通常Gitへコミットしません。

- `target/`
- `libadbc_driver_bigquery.dylib`
