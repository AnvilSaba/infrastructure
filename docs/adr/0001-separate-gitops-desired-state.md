---
status: accepted
---

# GitOps の desired state を専用リポジトリで管理する

アプリケーションの開発・成果物の公開と、環境へ配置するバージョンの決定を分けるため、GitOps の desired state を `AnvilSaba/platform` から `AnvilSaba/infrastructure` へ移す。
`platform` はアプリケーションコード、DB マイグレーション、コンテナビルド、Helm Chart ソースを所有し、`infrastructure` はクラスタ構成、環境別設定、SOPS で暗号化した Secret、配置する成果物の参照と更新処理を所有する。
環境への配置判断と履歴を desired state 側に集約し、成果物の検出と Git 更新には Flux の Image Automation を使用する。

## 環境別の構成

Flux の入口を `clusters/{shared,staging,production}/`、CloudNativePG などの基盤定義を `infrastructure/{base,staging,production}/`、アプリケーション定義を `apps/{base,staging,production}/platform/` に配置する。
環境別の app config と暗号化済み Secret、および暗号化ルールの `.sops.yaml` も `infrastructure` が所有する。
Chart ソースは `platform` に残し、配置対象の Chart と image の参照を `infrastructure` で管理する。

namespace は production が `anvilsaba`、staging が `anvilsaba-staging` を使う。
同じクラスタではノード、Flux controller、CNPG Operator を共有する。共通 controller・Operator・成果物の参照元は `clusters/shared` から管理する。DB・アプリ・更新ポリシーは各環境の入口で管理し、staging は共通基盤と staging の入口だけで構築できる。
staging の DB は共通 Operator に依存し、DB・PVC・アプリ用 Secret・Discord Bot・Cloudflare Tunnel は環境ごとに分ける。
SOPS の暗号化には全環境で同じ PQ 鍵を使う。
環境ごとにディレクトリと Flux の入口を分け、staging にはコンテナの既定のリソース制限と ResourceQuota を設定する。namespace を分けても環境間通信やノード障害の影響は分離されない。production のデータや資格情報は staging に使用しない。

## 成果物の公開と配置の更新

同じクラスタの Flux が両環境の成果物の検出、Git の自動更新、デプロイを行う。Kustomization と配置設定は環境ごとに分ける。
staging を別クラスタに配置する場合も、そのクラスタに共通基盤と staging の入口を適用する。同じ環境の Git 自動更新は一つのクラスタだけで有効にする。
staging のワークロードを停止していても、Git の自動更新は継続する。
SOPS 復号鍵は全環境で共有し、配置先クラスタの `flux-system` に Secret `sops-age` として配置する。

staging は application image・db-migrator・OCI Chart の可変タグ `main` の digest を追跡し、`apps/staging/platform/` の更新候補を `automation/staging` ブランチの1つの PR に集約する。
main の自動更新コミットを減らすため、必要なタイミングで人が squash merge し、取り込むまで staging の配置は維持する。
並び替え用の時刻・連番タグ、公開通知、pending 状態、Git の祖先判定、アプリと Migrator の事前互換性判定は設けない。
各アプリは既存の起動時チェックで必要な migration の適用有無・成功状態・checksum を検証する。
公開タイミングのずれによる staging の起動失敗は許容し、後続成果物の反映や修正で解消する。

production は application image と Chart の安定版 SemVer、db-migrator の可変タグ `release` を追跡する。
Flux は `apps/production/platform/` の更新を専用ブランチに push し、小さな GitHub Actions が既存 PR を再利用して提案する。
成果物の公開は更新候補の検出につながるが、production の desired state は人が PR をレビューして merge するまで変わらない。
release workflow の通知と任意の deploy 入力、独自の release/Git revision 照合スクリプトは廃止する。
staging の停止中も候補を作成できるが、Git の更新だけでは動作確認済みとは扱わない。
アプリ・Migrator の組み合わせと DB の後方互換性はレビューし、未確認なら staging を起動して確認する。

## 成果物の識別と更新順序

staging / production とも、application image・db-migrator・OCI 形式の Helm Chart は digest で配置対象を固定する。
Git の source revision、SHA tag、release tag は追跡情報として残し、ソースの識別と完成した成果物の識別を区別する。
digest 対応に必要な Chart の改修は `platform` で行い、既存環境の source 移管とは分ける。

`main` の同じ component のビルドは共通の concurrency group に入れ、新しい実行で古い実行をキャンセルする。
release 用ビルドはこのキャンセル対象から分離する。
可変タグはレジストリで現在公開されている内容を指し、Git の祖先関係を保証しない。
Flux はタグから解決した digest を Git に記録する。再起動だけで取得内容が変わらないよう、タグだけの配置は避ける。

意図した production アプリの切り戻しは手動 PR で行い、Migrator は以前の revision へ戻さない。
アプリだけを切り戻す場合は Migrator の配置を維持する。Flux による上書きを止めるため、対象 policy の選択範囲も PR で固定する。
Flux の標準機能と公開チャネルの選択に関する根拠は [公式仕様](https://fluxcd.io/flux/components/image/imagepolicies/#digest-reflection) を参照する。

## リポジトリ間の認証

Git 更新には infrastructure のみに Contents: write を持つ GitHub App を使用する。
PR 作成には Pull requests: write を付与した App token を使用し、PR の CI が自動起動する経路を維持する。
App には main の bypass を与えず、両環境とも PR と必須 CI を経由して取り込む。署名必須化と Flux の署名生成・検証は保留する。
GHCR は公開パッケージを前提として匿名で読み取り、読み取り用 token や platform の source 読み取り用 App は要求しない。
platform には GHCR への公開に必要な通常の Actions 権限だけを持たせる。

## 既存環境の引き継ぎ

既存の source が稼働している場合に限り、以下の引き継ぎ手順を適用する。

両環境の desired state と Flux 定義を準備してから、Flux の `GitRepository` と各 `Kustomization.spec.path` を新しいリポジトリへ切り替える。
切り替えでは既存 resource と新しい定義の対応を確認し、Flux が同一 resource の管理を継続できるようにする。
既存 resource の識別情報と管理関係を維持し、旧定義の削除による prune と、新旧の同期による競合を避ける順序で移行する。
管理元を切り替える際は、停止設定の解除、配置バージョンの変更、DB の切り替えを行わず、既存環境の状態を引き継ぐ。
移行完了後は `infrastructure/main` を staging / production の desired state の正とする。
