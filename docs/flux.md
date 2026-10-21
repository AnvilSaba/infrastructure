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

`$gitopsAppId` に App ID、`$gitopsAppKeyPath` に Git 管理外の秘密鍵ファイルのパスを設定します。
次のコマンドは、暗号化済みファイルがまだない場合にだけ実行します。平文の Secret はファイルへ保存しません。

```powershell
$secretPath = 'config/shared/secrets/gitops-update.sops.yaml'
if (Test-Path $secretPath) { throw '既存の Secret は SOPS のエディターで更新してください' }
$gitopsSecret = mise exec -- kubectl -n flux-system create secret generic gitops-update `
  --from-literal=githubAppID=$gitopsAppId `
  --from-literal=githubAppInstallationOwner=AnvilSaba `
  --from-file=githubAppPrivateKey=$gitopsAppKeyPath --dry-run=client -o yaml
if ($LASTEXITCODE -ne 0) { throw 'Secret の生成に失敗しました' }
$gitopsSecret | mise exec -- sops encrypt --filename-override $secretPath `
  --input-type yaml --output-type yaml --output $secretPath
if ($LASTEXITCODE -ne 0) { throw 'SOPS 暗号化に失敗しました' }
Remove-Variable gitopsSecret
```

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
