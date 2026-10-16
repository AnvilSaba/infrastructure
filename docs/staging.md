# staging の構築と自動更新

staging は `anvilsaba-staging` namespace を使用する。Cluster 名は同じでも namespace が異なるため
production の DB とは別物になる。StorageClass は `platform-staging-db-retain`、DB は新規 initdb とし、
legacy import や production Secret は参照しない。アプリ・DB 用 Secret、Discord Bot、テスト用 guild、
Cloudflare Tunnel と hostname を staging 専用に用意する。

## 初期構築

1. 対象クラスタの kubeconfig を明示的に選ぶ。別クラスタでは k3s と local-path provisioner を準備する。
2. Flux と CNPG Operator を共通定義から導入する。同じクラスタで既に導入済みなら再導入しない。
   `mise exec -- kubectl apply --server-side -k infrastructure/base/controllers`、
   `mise exec -- kubectl apply --server-side -k infrastructure/base/operator` を使い、それぞれ rollout を確認する。
   これらは production の同期定義や DB の稼働を要求しない。
3. staging 専用 age 秘密鍵を安全な場所に保管する。今回ローカルで生成した `staging.agekey` は Git 対象外。
   `.sops.yaml` の staging recipient だけが公開情報として記録される。
   別の鍵に変える場合は recipient と暗号化済み Secret を一緒に更新する。
4. `mise exec -- kubectl -n flux-system create secret generic sops-age-staging --from-file=age.agekey=staging.agekey`
   で専用鍵を対象クラスタに配置する。production の `sops-age` は使用しない。
5. `config/samples/` から staging 専用の値を設定した `*.plain.yaml` を作り、
   `mise exec -- sops --encrypt --filename-override config/staging/secrets/bot.sops.yaml <入力>` のように暗号化する。
   `*-database` の username/password はサンプルに従う。生成した `*.sops.yaml` のみを登録し、
   `config/staging/secrets/kustomization.yaml` の resources に追加する。平文と秘密鍵は登録しない。
6. `config/staging/bot.toml` のサンプル ID/hostname を staging の値に置換する。
7. `mise exec -- kubectl apply -k clusters/staging` で入口を用意する。
   初期状態は config/DB/HelmRelease を停止している。Secret、外部接続先、全成果物の準備が済んだら、
   config、DB、HelmRelease の順に Git の suspend を解除し、各依存先の Ready を確認する。
   自動更新処理は停止設定を解除しない。

## Flux による自動更新

両環境の成果物監視と Git 更新は、production クラスタの
`infrastructure/production/image-automation/` に集約する。
staging 自体は停止していてもよく、再起動したときに staging 側の Flux が最新の desired state を反映する。
production 側に staging の kubeconfig や復号鍵を渡さない。

platform/main のビルドは application image、db-migrator、Chart の可変タグ `main` を更新する。
Flux の ImagePolicy はこの一つのタグだけを対象とし、`digestReflectionPolicy: Always` で digest の変更を追う。
ImageUpdateAutomation は `apps/staging/platform/` の digest を main に直接 commit する。
タグだけで配置せず、解決した digest を Git に固定する。Chart の `0.0.0-main` は Helm 公開用の固定バージョンであり、
時刻・連番付きのタグを増やさない。Git revision は成果物の OCI metadata と既存 SHA タグから追跡できる。

公開通知、platform の clone、artifacts.json、祖先判定、Migrator の事前互換性判定は使用しない。
各アプリの既存の起動時チェックが、必須 migration の適用・成功・checksum を照合する。
公開タイミングがずれて起動に失敗しても、後続の更新や修正で解消する。
staging を停止したまま Git が更新されても、動作検証済みにはならない。

## 認証と有効化

GHCR は Public パッケージを匿名で監視する。読み取り用 Secret や platform 側の通知用 App は不要。
Git 更新用 App は infrastructure のみにインストールし、Contents: write を付与する。
production PR 用には Pull requests: write も必要になる。

production クラスタの flux-system に `gitops-update` Secret を作成する。
以下の変数には App ID と秘密鍵ファイルのパスを設定する。秘密鍵は Git に登録しない。

```powershell
mise exec -- kubectl --context <production-context> -n flux-system create secret generic gitops-update `
  --from-literal=githubAppID=$gitopsAppId `
  --from-literal=githubAppInstallationOwner=AnvilSaba `
  --from-file=githubAppPrivateKey=$gitopsAppKeyPath
```

これは Flux の GitRepository provider: github が短期 token を取得するための Secret である。
[Flux の GitHub App 設定](https://fluxcd.io/flux/components/source/gitrepositories/#github)に従う。
App の push で通常の CI が起動するようにし、main の保護設定では staging の参照更新を許可する。

source 移管後、GitRepository `platform-image-updates` と各 ImagePolicy の Ready を確認する。
`staging.yaml` の ImageUpdateAutomation `platform-staging` の suspend を Git で解除する。
初回の全 digest が埋まり、digest 対応済み Chart が反映されるまでは HelmRelease を停止したままにする。
ImageUpdateAutomation の suspend と staging の HelmRelease の suspend は別の設定である。
前者だけを有効にすれば staging を停止したまま Git の参照を更新できる。

## 受け入れ時の確認

App installation/資格情報、branch protection、実 package の公開チャネル、Flux の実更新は実環境で確認する。
staging の鍵の保管・暗号化 Secret・外部設定を完了してから、Flux の Git commit と CI、
source/Kustomization の Ready、復号、Cluster/DatabaseRole、HelmRelease の Ready、
Bot/紐付け/Public API/Tunnel の疎通を確認する。描画・commit 成功だけでは実反映完了にしない。
production source の切り替えと旧定義の削除は Issue #6 で行う。
