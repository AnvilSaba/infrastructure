# Flux の初期構築

クラスタへの初回接続と、成果物の Git 自動更新に必要な認証を設定します。
環境ごとの構築は [production](deployment.md) または [staging](staging.md) の手順に従ってください。

## 初回の接続

対象クラスタの kubeconfig を選択し、リポジトリルートで実行します。適用する構成は先に `infrastructure/main` に反映します。
Flux controller と CNPG Operator が未導入の場合は、次を実行します。

```powershell
mise exec -- kubectl apply --server-side -k infrastructure/base/controllers
mise exec -- kubectl apply --server-side -k infrastructure/base/operator
mise exec -- kubectl -n flux-system rollout status deployment/source-controller --timeout=5m
mise exec -- kubectl -n flux-system rollout status deployment/kustomize-controller --timeout=5m
mise exec -- kubectl -n cnpg-system rollout status deployment/cnpg-controller-manager --timeout=5m
```

[Git 更新用 App](#git-更新用-app)の手順に従い、共通基盤の Secret と復号鍵を準備します。
共通基盤の入口を適用します。同じクラスタでは一度だけ設定し、両環境で共有します。

```powershell
mise exec -- kubectl apply --server-side -k clusters/shared
mise exec -- kubectl -n flux-system wait kustomization/flux-controllers kustomization/cnpg-operator --for=condition=Ready --timeout=5m
mise exec -- kubectl -n flux-system wait kustomization/infrastructure-config gitrepository/platform-image-updates --for=condition=Ready --timeout=5m
```

`clusters/shared` は共通の controller・Operator・成果物の参照元と Git 更新用の Secret を管理します。環境ごとの DB・アプリ・更新ポリシーは含みません。

[Secret の手順](secret-config.md#flux-への鍵の配置)に従い、復号鍵を格納した Secret `sops-age` が配置済みであることを確認します。その後、対象環境の入口を適用します。次は staging の例です。

```powershell
mise exec -- kubectl apply --server-side -k clusters/staging
```

production を構築する場合は `clusters/production` を指定します。staging だけを構築する場合、production の入口は不要です。

staging は Config・DB・DB ロール・アプリの順に自動で反映されます。
production は Config・DB・HelmRelease と ImageUpdateAutomation の停止設定を維持し、環境の構築手順に従って解除します。既存 DB がある場合は [DB 移行手順](cloudnativepg-migration.md)へ進んでください。

## 基盤マニフェストとイメージの更新

Flux と CNPG の公式配布物を、それぞれ `infrastructure/base/controllers/upstream.yaml` と `infrastructure/base/operator/upstream.yaml` に原文のまま保存します。イメージの固定と起動オプションは同じディレクトリの `kustomization.yaml` で指定します。保存した配布物は整形・改行変換の対象から除外しています。

| 配布物       | 取得元                                                                                                                                                            | SHA-256                                                            |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| Flux v2.9.6  | [公式 install.yaml](https://github.com/fluxcd/flux2/releases/download/v2.9.6/install.yaml)                                                                        | `9c1fda7e401429531ed1478f67ba06b3edff6513b75da7ee47bde9f1c4d4251c` |
| CNPG v1.30.1 | [検証済み commit の manifest](https://raw.githubusercontent.com/cloudnative-pg/cloudnative-pg/2a35abb4628f209d149825ef3c38011e0701ff2f/releases/cnpg-1.30.1.yaml) | `37237f145d8138256ea25ae830f87759255665ff08f8d552fdd8224a5ec032fb` |

更新時は次の順で進めます。

1. 公式リリースの manifest・署名・checksums を一時ディレクトリに取得し、期待する repository・workflow・バージョンを指定して署名を検証します。Flux の manifest は署名済み checksums にある `install.yaml` のチェックサムと照合し、CNPG は manifest を対応する `.sigstore.json` で検証します。上流の変更に応じて署名 identity も公式 workflow と照合します。[Flux の署名仕様](https://fluxcd.io/flux/security/)、[CNPG の検証手順](https://cloudnative-pg.io/docs/1.30/installation_upgrade/#verifying-release-assets)を参照してください。
2. 検証したファイルで `upstream.yaml` を置き換え、この表の取得元・バージョン・SHA-256 を更新します。SHA-256 は `Get-FileHash <ファイル> -Algorithm SHA256` で確認できます。
3. Flux の全7画像と CNPG Operator の画像について、`images` のタグ・digest を更新します。併せて `infrastructure/base/database/cluster.yaml` の PostgreSQL のタグ・digest を更新します。OCI index の完全な digest を使い、ノードのアーキテクチャに対応する manifest が含まれることを確認します。index の digest と稼働 Pod の platform 別 imageID は、そのまま文字列比較しません。
4. 各画像をタグではなく digest 指定で署名検証します。例えば CNPG Operator は取得した digest を設定して次のように確認します。略記 digest は使いません。

   ```powershell
   $operatorImage = 'ghcr.io/cloudnative-pg/cloudnative-pg@sha256:<完全なdigest>'
   $operatorIdentity = 'https://github.com/cloudnative-pg/cloudnative-pg/.github/workflows/release-publish.yml@refs/tags/v<対象版>'
   mise exec aqua:sigstore/cosign@3.1.4 -- cosign verify $operatorImage --certificate-identity $operatorIdentity --certificate-oidc-issuer https://token.actions.githubusercontent.com
   ```

5. `mise run fmt`、`mise run fmt:check`、`mise run check` を実行します。描画後の画像一覧に未固定の参照がないこと、リソースの識別情報や権限・DB 設定に意図しない変更がないことを確認し、PR でレビューします。CNPG は `OPERATOR_IMAGE_NAME` を Operator の固定画像から自動で置換し、DB Pod の初期化画像も同じ digest にします。
6. 共有コントローラは production にも影響します。稼働中の画像と採用する画像を照合し、DB のバックアップ・復元手順と作業時間を確認してから反映します。DB が1インスタンスのため、イメージ参照や初期化画像の変更で再起動・停止が発生し得ます。production DB の停止設定の解除は別の作業として扱います。

kustomize-controller は `--no-remote-bases=true` を指定しており、Kustomize の `resources` には source artifact 内のローカルファイルを指定します。今回の移行でリポジトリ内の外部 manifest 参照は解消しています。別の GitRepository 等も同じ controller で管理している場合は、そちらも外部 base を使っていないことを反映前に確認してください。切り戻し時もローカルファイル参照を維持し、以前の検証済み画像・設定へ戻します。[Flux のフラグ仕様](https://fluxcd.io/flux/components/kustomize/options/)

PostgreSQL の rolling tag の変化は更新候補として扱い、digest の不一致だけで侵害と判断して一律停止しません。固定後は上流のセキュリティ修正を自動採用しないため、リリース通知や定期点検をもとに検証し、更新 PR を継続して作成します。

## Git 更新用 App

Flux が Git を読み取り、変更を push するための GitHub App を infrastructure にインストールし、Contents: write を付与します。
App ID と発行した秘密鍵ファイルを用意します。秘密鍵ファイルは Git 管理外で保管し、平文では登録しません。
`gitops-update` Secret は SOPS で暗号化して `config/shared/secrets/` に保存し、Flux が `flux-system` に配置します。
共通基盤の読み取り用 GitRepository は App の認証を使わないため、Secret の配置に Git 更新用の認証は必要ありません。

### 復号鍵

production・staging・共通基盤で1つの PQ 鍵を使います。新しい管理端末ではバックアップから復元します。
[Secret の設定手順](secret-config.md#設定の更新)に従って `mise.local.toml` を設定します。

秘密鍵は本人だけが読める権限にし、Git 管理外にバックアップします。
controller の導入後、[Flux への鍵の配置](secret-config.md#flux-への鍵の配置)に従って `sops-age` Secret を配置します。同じクラスタでは一度だけ実行します。

### App の Secret の登録

[`gitops-update.sample.yaml`](../config/samples/gitops-update.sample.yaml) から暗号化済みファイルを作り、SOPS のエディターで実値を入力します。サンプル自体には実値を書き込みません。
次のコマンドは、暗号化済みファイルがまだない場合にだけ実行します。

```powershell
$secretPath = 'config/shared/secrets/gitops-update.sops.yaml'
if (Test-Path $secretPath) { throw '既存の Secret は SOPS のエディターで更新してください' }
Get-Content 'config/samples/gitops-update.sample.yaml' -Raw |
  mise exec -- sops encrypt --filename-override $secretPath --input-type yaml --output-type yaml --output $secretPath
if ($LASTEXITCODE -ne 0) { throw 'SOPS 暗号化に失敗しました' }
mise run sops:edit $secretPath
```

エディターで `githubAppID` に App ID を文字列として入力し、`githubAppPrivateKey` の `|` 以下に秘密鍵ファイルの PEM 全文をインデントして貼り付けます。`githubAppInstallationOwner` は `AnvilSaba` のままにします。保存して終了すると再暗号化されます。

生成したファイルを `config/shared/secrets/kustomization.yaml` の `resources` に追加します。
暗号化済み Secret と設定を `infrastructure/main` に反映すると、Kustomization `infrastructure-config` が Secret を復号して配置します。
`gitops-update` Secret をすでに手動で作成している場合は、同じ資格情報を暗号化して登録し、Flux が配置する前に既存の Secret を削除しません。
App が未作成の間は `resources: []` を維持します。この間は Git の自動更新を利用できません。

更新時は次のコマンドで編集します。

```powershell
mise run sops:edit config/shared/secrets/gitops-update.sops.yaml
```

main は PR 経由で保護し、App には staging の参照更新に必要な範囲で直接 push を許可します。
`automation/production` への App の push / force-push も許可します。
App の push で CI が起動することを確認してください。

PR 作成用の認証は [production 更新 PR](production-pr.md#pr-作成用-app)で設定します。

自動更新の設定は [staging](staging.md#自動更新の設定) と [production](production-pr.md#初回有効化) の手順に従ってください。

## 反映の確認

対象 Kustomization の `status.lastAppliedRevision` が対象コミットを示すことを確認します。
HelmRelease では `status.observedGeneration` が `metadata.generation` に追いつき、
Ready と配置した image の digest が意図した値になることを確認します。
Git の更新や描画だけではデプロイ完了とは扱いません。各環境の手順に従って疎通も確認します。
