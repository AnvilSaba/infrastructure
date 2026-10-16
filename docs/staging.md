# staging の構築と自動更新

staging は `anvilsaba-staging` namespace を使用する。Cluster 名は同じでも namespace が異なるため
production の DB とは別物になる。StorageClass は `platform-staging-db-retain`、DB は新規 initdb とし、
legacy import や production Secret は参照しない。アプリ・DB 用 Secret、Discord Bot、テスト用 guild、
Cloudflare Tunnel と hostname を staging 専用に用意する。

## 初期構築

1. 対象クラスタの kubeconfig を明示的に選ぶ。別クラスタでは k3s と local-path provisioner を準備する。
2. Flux と CNPG Operator を共通定義から導入する。同じクラスタで既に導入済みなら再導入しない。
   `mise exec -- kubectl apply -k infrastructure/base/controllers`、
   `mise exec -- kubectl apply -k infrastructure/base/operator` を使い、それぞれ rollout を確認する。
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

## 公開通知

platform/main の対象 image の公開成功後に `platform-artifact` を通知する。
component、repository、digest、実際の revision、SHA tag を必須とし、Chart は `0.0.0-sha.<revision>` を使う。
受信側は許可した GHCR repository の manifest/config を読み、digest と OCI revision を照合する。
Chart のソース revision も OCI annotation で照合し、固定した chart archive を lint/template する。

通知と pending/deployed の追跡は `apps/staging/platform/artifacts.json` に保存する。
アプリの REQUIRED_MIGRATIONS にある SQL を、公開済み Migrator の revision と Git blob で照合する。
必須 migration が同一ならアプリだけを更新し、足りないときだけ Migrator を待つ。到着順によらず対象だけを drain し、
初回から無関係な component を待たず、準備できた参照だけを書き出す。
Chart と全 image が揃った時点で組み合わせを lint/template する。初期構築中は HelmRelease を停止したままにする。
同じ revision と digest は変更なし、祖先からの古い通知は据え置き、分岐・不明な履歴・同 revision の別 digest は失敗する。
通知を受けるたび最新 main に対して計算し、push 競合では最新 main を読み直して最大5回再検証する。
GitHub の workflow concurrency で異なる component の pending 通知を捨てない。

## 用途別の認証

- platform の通知用 App: infrastructure のみ、Contents: write。
  `GITOPS_NOTIFY_APP_ID` (variable)、`GITOPS_NOTIFY_APP_PRIVATE_KEY` (secret)。
- infrastructure の更新用 App: infrastructure のみ、Contents: write。production PR では Pull requests: write も必要。
  `GITOPS_UPDATE_APP_ID`、`GITOPS_UPDATE_APP_PRIVATE_KEY`。
- infrastructure の source 検証用 App: platform のみ、Contents: read。
  `GITOPS_READ_APP_ID`、`GITOPS_READ_APP_PRIVATE_KEY`。
- private GHCR を使う場合は用途を読み取りに限定した `GITOPS_REGISTRY_READ_TOKEN` と `GITOPS_REGISTRY_USER` を設定する。
  公開 package は匿名で読めるため省略できる。

通知用 App にも書き込み権限がある。更新用と秘密鍵を共用せず、インストール対象を限定する。
生成 commit/PR は App token で作り、通常の CI が起動する経路にする。

## 受け入れ時の確認

専用資格情報、App installation/権限、branch protection、package 閲覧権限は未設定・未検証。
staging の鍵の保管・暗号化 Secret・外部設定を完了させてから、実通知で生成 commit と GitOps CI の起動、
Flux source/Kustomization の Ready、復号、Cluster/DatabaseRole、HelmRelease の Ready、
Bot/紐付け/Public API/Tunnel の疎通を確認する。描画・commit 成功だけでは実反映完了にしない。
production source の切り替えと旧定義の削除は Issue #6 で行う。
