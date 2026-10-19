# staging の構築と自動更新

staging 専用の Secret、Discord Bot、テスト用 guild、Cloudflare Tunnel と hostname を用意する。
production のデータや資格情報は使用しない。

## 初期構築

1. 対象クラスタの kubeconfig を選ぶ。別クラスタの場合は、先に k3s と local-path provisioner を準備する。
2. [Flux の初回接続](flux.md#初回の接続)に従い、共通 controller と CNPG Operator を導入する。同じクラスタで導入済みなら再導入しない。
3. [新規環境の Secret 準備](secret-config.md#新規環境の準備)に従い、staging 専用の鍵と Secret、通常設定を用意する。対象クラスタの `flux-system` に復号鍵を `sops-age-staging` として配置する。
4. `mise exec -- kubectl apply -k clusters/staging` で接続する。Config・DB・HelmRelease の停止設定は維持する。
5. 次節の自動更新を有効にし、`apps/staging/platform/` の Chart と全 image に公開済みの digest が入ったことを確認する。
6. `clusters/staging/config.yaml` の `spec.suspend` を Git で `false` にし、`staging-platform-config` の Ready を確認する。
7. `clusters/staging/database.yaml` の `staging-platform-db.spec.suspend` を Git で `false` にし、`staging-platform-db` と `staging-platform-db-roles` の Ready を確認する。
8. `apps/staging/platform/release.yaml` の `spec.suspend` を Git で `false` にし、HelmRelease の Ready を確認する。

## 自動更新の有効化

[Flux の Git 更新用 App](flux.md#git-更新用-app)を production クラスタに設定する。
`platform-image-updates` と staging の各 ImagePolicy が Ready になったら、
`infrastructure/production/image-automation/staging.yaml` の ImageUpdateAutomation `platform-staging` の `spec.suspend` を Git で `false` にする。

以降は platform/main の成果物が `apps/staging/platform/` に自動反映される。
staging を停止していても Git の更新は続き、再起動時に最新の設定が反映される。
Git の更新だけでは動作確認済みにはならない。

## 反映の確認

[Flux の反映確認](flux.md#反映の確認)に加え、次を確認する。namespace は `anvilsaba-staging` を使う。

- Config の復号、Cluster と DatabaseRole、HelmRelease が正常である
- Bot が Discord に接続し、リンクコードの発行・連携・解除ができる
- staging の Tunnel 経由で `/whitelist.json` を取得できる
