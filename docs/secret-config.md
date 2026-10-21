# Secret / Config の準備と運用

production の通常設定と暗号化済み Secret は登録済みです。既存の設定を更新する場合は、鍵や Secret を作り直さず「設定の更新」へ進んでください。
新しい環境には「新規環境の準備」を使います。既存 DB の切り替えは [DB 移行手順](cloudnativepg-migration.md)を参照してください。
Git 更新用 App の Secret は環境別の Secret と分け、[共通基盤の手順](flux.md#git-更新用-app)で管理します。

コマンド例は PowerShell 7 用です。mise と kubeconfig を設定した管理端末で、リポジトリルートから実行します。秘密鍵は Git 外で保管・バックアップし、本人だけが読めるファイル権限にします。

## 設定の更新

production・staging・共通基盤で使う1つの PQ 秘密鍵を指定します。

```powershell
$ageKeyFile = Join-Path $env:USERPROFILE '.config/sops/age/keys-pq.txt'
$env:SOPS_AGE_KEY_FILE = $ageKeyFile
```

通常設定を変更するには、`config/<環境>/bot.toml` と `apps/<環境>/platform/environment-values.yaml` を編集します。Token やパスワードは追加しません。
Secret は SOPS のエディターで更新します。

```powershell
mise exec -- sops edit config/production/secrets/bot.sops.yaml
```

必要なら `$env:SOPS_EDITOR = 'code --wait'` などでエディターを指定します。暗号文・SOPS メタデータ・平文の名前や namespace を通常のエディターで直接変更しません。

Git へ反映すると Flux が同期します。ConfigMap と環境変数の Secret を更新しても Pod は再起動しないため、更新した設定を使うワークロードを再起動します。

```powershell
mise exec -- kubectl -n anvilsaba rollout restart deployment/bot
mise exec -- kubectl -n anvilsaba rollout status deployment/bot --timeout=5m
```

DB パスワードを更新する場合は、対応するアプリを先に停止します。更新後に DatabaseRole の `status.applied` が `true` であり、`status.secretResourceVersion` が Secret の `metadata.resourceVersion` と一致することを確認してから、アプリを再起動します。以前の `applied=true` だけで完了と判断しません。

## 新規環境の準備

### 復号鍵

全環境で1つの PQ 鍵を使います。秘密鍵は Git 管理外で保管し、新しい管理端末ではバックアップから復元します。
`.sops.yaml` は production・staging・共通基盤で同じ recipient を参照します。
鍵を変更する場合は、全環境の暗号化済み Secret をすべて更新します。

### Secret と通常設定

`config/samples/*.sample.yaml` から必要な Secret を作成します。サンプルの namespace を対象環境に合わせ、置換用の値は適用前に実値へ変更します。

| Secret 名                 | 用途                          |
| ------------------------- | ----------------------------- |
| `bot`                     | Bot Token と認証の合言葉      |
| `bot-database`            | Bot の DB 資格情報            |
| `mc-link-server-database` | MC Link Server の DB 資格情報 |
| `public-api-database`     | 公開 API の DB 資格情報       |
| `db-migrator-database`    | Migrator の DB 資格情報       |
| `cloudflared`             | Tunnel Token                  |

DB 専用 Secret の `username` はサンプルに従い、`password` に対象環境の値を設定します。DB 専用 Secret に秘密項目を追加するときは、`.sops.yaml` の暗号化対象も更新します。

次は staging の Bot Secret を作成する例です。既存ファイルには実行しません。`SOPS_AGE_KEY_FILE` に共通の PQ 秘密鍵のパスを設定してから実行します。

```powershell
$secretName = 'bot'
$secretPath = "config/staging/secrets/$secretName.sops.yaml"
Get-Content "config/samples/$secretName.sample.yaml" -Raw |
  mise exec -- sops encrypt --filename-override $secretPath --input-type yaml --output-type yaml --output $secretPath
if ($LASTEXITCODE -ne 0) { throw 'SOPS 暗号化に失敗しました' }
mise exec -- sops edit $secretPath
```

必要な Secret ごとに実行し、生成したファイルを `config/<環境>/secrets/kustomization.yaml` の `resources` に追加します。
`config/<環境>/bot.toml` のサンプル ID や hostname も対象環境の値へ変更します。

```powershell
mise exec -- kustomize build config/staging
```

Git に追加する前に、必要な Secret が登録され、暗号化対象に平文の実値が残っていないことを確認します。暗号化した Secret を含む build の出力は、そのまま kubectl で適用しません。
平文の Secret 原本や秘密鍵は Git やノードのデプロイディレクトリに置きません。資格情報をコミットした場合は失効・再発行してください。

### 復号の確認

鍵を管理する本人が `SOPS_AGE_KEY_FILE` に秘密鍵のパスを指定し、次を実行します。復号結果は表示・保存しません。

```powershell
Get-ChildItem config/production/secrets,config/staging/secrets,config/shared/secrets -Filter '*.sops.yaml' |
  ForEach-Object {
    mise exec -- sops decrypt $_.FullName | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "復号に失敗しました: $($_.Name)" }
  }
```

### Flux への鍵の配置

[Flux の初回接続](flux.md#初回の接続)で controller を導入したあと、対象クラスタに鍵を配置します。同じクラスタでは一度だけ設定し、production・staging・共通基盤で共有します。

```powershell
mise exec -- kubectl -n flux-system create secret generic sops-age `
  --from-file=identity.agekey=$ageKeyFile --dry-run=client -o yaml |
  mise exec -- kubectl apply -f -
```

staging 単独のクラスタでも、同じ PQ 鍵を `flux-system` の Secret `sops-age` に配置します。
PQ 鍵の復号には対応した kustomize-controller が必要です。リポジトリで固定した Flux を使用してください。管理端末の age だけを更新しても controller の復号処理は更新されません。
