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

共通基盤の入口を適用します。同じクラスタでは一度だけ設定し、両環境で共有します。

```powershell
mise exec -- kubectl apply --server-side -k clusters/shared
mise exec -- kubectl -n flux-system wait kustomization/flux-controllers kustomization/cnpg-operator --for=condition=Ready --timeout=5m
```

`clusters/shared` は共通の controller・Operator・成果物の参照元を管理します。環境ごとの DB・アプリ・更新ポリシーは含みません。

[Secret の手順](secret-config.md#flux-への鍵の配置)に従って対象環境の復号鍵を配置し、その環境の入口だけを適用します。次は staging の例です。

```powershell
mise exec -- kubectl apply --server-side -k clusters/staging
```

production を構築する場合は `clusters/production` を指定します。staging だけを構築する場合、production の入口や復号鍵は不要です。

staging は Config・DB・DB ロール・アプリの順に自動で反映されます。
production は Config・DB・HelmRelease と ImageUpdateAutomation の停止設定を維持し、環境の構築手順に従って解除します。既存 DB がある場合は [DB 移行手順](cloudnativepg-migration.md)へ進んでください。

## Git 更新用 App

Flux が Git を読み取り、変更を push するための GitHub App を infrastructure にインストールし、Contents: write を付与します。
自動更新を実行するクラスタの `flux-system` に `gitops-update` Secret を作成します。
`$gitopsAppId` に App ID、`$gitopsAppKeyPath` に秘密鍵ファイルのパスを設定します。秘密鍵は Git に登録しません。

```powershell
mise exec -- kubectl --context <cluster-context> -n flux-system create secret generic gitops-update `
  --from-literal=githubAppID=$gitopsAppId `
  --from-literal=githubAppInstallationOwner=AnvilSaba `
  --from-file=githubAppPrivateKey=$gitopsAppKeyPath
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
