# staging の構築と更新 PR

staging は共通基盤の `clusters/shared` と環境設定の `clusters/staging` だけで構築できる。production の構築や稼働は必要ない。
Flux controller と CNPG Operator は共通基盤で管理する。同じクラスタではノードも共有し、DB・PVC・Secret・Discord Bot・テスト用 guild・Cloudflare Tunnel と hostname は環境ごとに分ける。
production のデータや資格情報は使用しない。

## 構成

| 項目                               | production           | staging                      |
| ---------------------------------- | -------------------- | ---------------------------- |
| namespace                          | `anvilsaba`          | `anvilsaba-staging`          |
| Flux の入口                        | `platform`           | `staging-platform`           |
| SOPS 復号鍵の Secret               | `sops-age`           | `sops-age`                   |
| DB の StorageClass                 | `platform-db-retain` | `platform-staging-db-retain` |
| MC Link Server の外向き TCP ポート | `25600`              | `25601`                      |

DB・Service・Helm release の名前が同じでも namespace は異なる。DB 接続先 `platform-db-rw` と Tunnel の転送先 `http://public-api:8080` は、それぞれの namespace 内で名前解決する。
staging の Tunnel は staging の `cloudflared` のみに接続し、API の公開ホスト名は `api.staging.anvilsaba.org` にする。公開ルートを作る前に Cloudflare Access で保護する。

ResourceQuota で staging 全体のコンテナの limit の合計を CPU 2 コア・メモリ 4 GiB に制限する。更新中の一時的な Pod と Migrator もこの上限に含む。Quota を超える Pod は作成されない。
この設定はリソースの予約や、本番専用の CPU コアの確保を行うものではない。

| 対象                       | CPU request | CPU limit | メモリ request | メモリ limit |
| -------------------------- | ----------- | --------- | -------------- | ------------ |
| 明示的な指定がないコンテナ | `100m`      | `250m`    | `128Mi`        | `512Mi`      |
| DB                         | `100m`      | `500m`    | `256Mi`        | `1Gi`        |

起動後に使用量・OOM・CPU throttling を確認し、必要に応じてこれらの初期値を調整する。

namespace の分離は通信の遮断を意味しない。現在は環境間通信を制限する NetworkPolicy を設けていない。
同じクラスタでは、ノードの障害、ディスク不足、共有 controller の障害が両環境に影響するため、起動前にノードの空き容量と既存 Pod の requests を確認する。

## 初期構築

1. 対象クラスタの kubeconfig を選び、`mise exec -- kubectl config current-context` で接続先を確認する。新しいクラスタでは、公開済み image に対応するノードで [k3s を準備](deployment.md#12-k3s)し、local-path provisioner が利用できることを確認する。
2. [Flux の初回接続](flux.md#初回の接続)に従い、共通基盤の `clusters/shared` を適用する。`flux-system` の Kustomization `flux-controllers` と `cnpg-operator` が Ready であることを確認する。
3. [新規環境の Secret 準備](secret-config.md#新規環境の準備)に従い、staging 専用の Secret と通常設定を用意する。全環境で共通の PQ 鍵を使い、復号鍵を格納した Secret `sops-age` が `flux-system` に配置済みであることを確認する。
4. [自動更新の設定](#自動更新の設定)に従い、Git 更新用 App の認証を配置する。
5. 配置設定を `infrastructure/main` に反映してから、初回のみ `mise exec -- kubectl apply --server-side -k clusters/staging` で staging の Flux の入口を追加する。以降は Git から同期される。

Config の同期後に DB、DB ロール、アプリの順に自動で反映される。DB は共通の CNPG Operator の Ready も待つ。
`staging-platform-config`、`staging-platform-db`、`staging-platform-db-roles` と HelmRelease の Ready を確認する。

## 自動更新の設定

[Flux の Git 更新用 App](flux.md#git-更新用-app)を対象クラスタに設定する。同じクラスタに設定済みの場合は再利用する。
staging の入口を適用すると自動更新も始まる。`platform-image-updates`、staging の各 ImagePolicy と ImageUpdateAutomation `platform-staging` の Ready を確認する。

platform/main で公開された成果物の更新候補は `automation/staging` ブランチに集約され、GitHub Actions の **Artifact update PR** が main 向けの PR を作成する。
認証は [共通の Git 更新用 App](flux.md#git-更新用-app)の設定に従う。
後続の更新候補も同じ PR に反映される。必要なタイミングで人が PR を squash merge すると、更新が staging に反映される。
同じ環境の自動更新は一つのクラスタで有効にする。

PR 作成に失敗した場合は、**Artifact update PR** を workflow_dispatch で実行し、`environment` に `staging` を選ぶ。
