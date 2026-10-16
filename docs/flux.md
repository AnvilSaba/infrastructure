# Flux の初期構築

クラスタ・環境別設定・配置対象は infrastructure を唯一の管理元にします。
platform は成果物の公開までを担当します。
準備中の GitOps 構成は未デプロイのため、platform の旧 source を稼働環境から切り替える作業は不要です。

## 初回の接続

対象クラスタの kubeconfig を選択し、リポジトリルートで実行します。

```powershell
mise exec -- kubectl apply --server-side -k infrastructure/base/controllers
mise exec -- kubectl apply --server-side -k infrastructure/base/operator
mise exec -- kubectl -n flux-system rollout status deployment/source-controller --timeout=5m
mise exec -- kubectl -n flux-system rollout status deployment/kustomize-controller --timeout=5m
mise exec -- kubectl -n cnpg-system rollout status deployment/cnpg-controller-manager --timeout=5m
```

[Secret の準備](secret-config.md)に従って production 用の復号鍵を配置し、入口を適用します。

```powershell
mise exec -- kubectl apply --server-side -k clusters/production
```

初期状態の config・DB・HelmRelease と ImageUpdateAutomation は停止したままです。
Secret、外部サービス、DB の準備、Chart と全 image の digest を確認してから、
config、DB、HelmRelease の順に Git の suspend を解除し、それぞれの Ready を確認します。
production の DB 定義は既存 PostgreSQL からの import を含むため、既存 DB がある場合は
[移行手順](cloudnativepg-migration.md)を確認します。空の DB を新規構築するときは、適用前に
`infrastructure/production/database/kustomization.yaml` の import patch を外して base の initdb を使用します。

## 成果物の更新

production のクラスタが両環境の公開成果物を監視します。
GitHub App の準備と staging の自動更新は [staging の手順](staging.md)、
承認用 PR と production の有効化は [production 更新 PR](production-pr.md)を参照してください。
staging 自体の Flux は独立して構築します。

## 反映の確認

対象 Kustomization の `status.lastAppliedRevision` が今回のコミットを示すことを確認します。
HelmRelease では `status.observedGeneration` が `metadata.generation` に追いつき、
Ready と配置した image の digest が意図した値になることを確認します。
描画や Git 更新だけではデプロイ完了とは扱いません。
