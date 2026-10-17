# production 更新 PR

production クラスタの Flux が、staging / production の成果物監視と Git 更新を担当する。
各環境の Flux は自身のクラスタへの反映だけを担当するため、staging が停止していても更新候補を準備できる。

## 公開から反映まで

1. platform の Release がアプリ・Migrator または Chart を GHCR に公開する。
2. Flux の ImagePolicy がアプリと Chart の安定版 SemVer、Migrator の可変タグ `release` を検出する。
3. ImageUpdateAutomation `platform-production` が `automation/production` ブランチの
   `apps/production/platform/` の参照を更新する。ブランチがなければ main から作成し、
   既存なら先端を再利用する。image と Chart は digest で固定し、変更がある場合だけ commit / push する。
4. **Production update PR** workflow が一つの PR を作成する。既存の PR がある場合はその PR を使う。
5. 人が CI、差分、staging の実行結果、アプリと Migrator の組み合わせ、SQL の後方互換性を確認して merge する。
   merge 後は更新ブランチを削除し、次の候補を最新 main から作成できるようにする。
6. production 側の Flux が main の変更を反映する。自動 merge や production の直接更新は行わない。

アプリと Chart は独立して最新版を選ぶ。全成果物を同じ Git revision に揃える制約や、
Git 祖先・SQL の一致を判定する更新スクリプトは設けない。
各アプリの起動時 migration チェックを維持する。staging の停止中に更新候補を検出しても動作確認済みではないため、
必要なら staging を起動して確認してから承認する。
Migrator の `release` はアプリ release の Migrator ビルド成功時に更新され、staging の `main` と区別する。
アプリと Migrator は別々に公開されるため、PR が作成された時点で必要な成果物がすべて揃っているとは限らない。

## 認証と初回有効化

Git 更新用 App と `gitops-update` Secret は [staging の認証設定](staging.md#認証と有効化)を参照する。
PR 作成 workflow では `GITOPS_UPDATE_APP_ID` variable と `GITOPS_UPDATE_APP_PRIVATE_KEY` secret を設定する。
App は infrastructure のみにインストールし、Contents: write と Pull requests: write を付与する。
workflow は自リポジトリが既定のため owner / repositories を列挙せず、PR 作成に必要な権限だけを要求する。
App token を使うことで生成 PR の CI を自動起動する。
GHCR は Public を匿名で読み取り、platform の通知用 App / source 読み取り用 App は不要。

source 移管と新版の公開を同時に行わない。
初期の ImageUpdateAutomation は suspend: true とし、既存 Chart/image と HelmRelease の停止状態を維持する。
Issue #6 の source 移管が完了してから、digest 対応 Chart の安定版を公開する。
Chart の policy は最初の digest 対応 release に合わせて `>=1.0.1` とし、旧版 `1.0.0` を候補にしない。
各 ImagePolicy の Ready を確認したら、`production.yaml` の `platform-production` の suspend を Git で解除する。
初回 PR では Chart と全 image の digest が埋まり、意図した版を示していることを確認する。

main は PR 経由で保護し、staging の参照を直接更新する App だけを必要な範囲で許可する。
`automation/production` への App の push / force-push と、承認後の人による merge を許可する。
更新後の古い承認を無効にする branch protection を設定する。
自動 PR 作成は merge の承認を代行しない。

## 手動指定と切り戻し

特定版や複数 component をまとめて指定するときは、main から別ブランチを作り、
`apps/production/platform/values.yaml` / `chart.yaml` の tag と対応する公開済み digest を PR で変更する。
変更しない component の参照は維持する。
Flux の選択範囲も同じ PR で固定する。アプリ / Chart は対象 ImagePolicy の SemVer range を特定版へ絞り、
Migrator を固定するときは filterTags を既存の完全 SHA タグへ変更して digestReflectionPolicy を IfNotPresent にし、interval を削除する。
アプリ切り戻しでは Migrator の digest を維持する。DB のロールバックは行わず、旧アプリとの後方互換性を確認する。
自動追従を再開するときは policy の範囲を戻す。

Flux は既存の更新ブランチを再利用する。`GitAllBranchReferences` は既定の true を使い、
同じ候補での再同期では新しい commit / push を発生させない。
[採用版 Flux の動作](https://github.com/fluxcd/image-automation-controller/blob/v1.2.5/internal/source/git.go)を参照する。
main 側の変更は PR の merge 時に統合する。競合する場合は更新ブランチへ main を merge し、
最新の CI と差分を再確認する。`automation/production` の成果物参照は手作業で編集しない。
PR の自動作成に失敗した場合は、ブランチの差分を確認したうえで workflow_dispatch から再実行できる。

## 未確認事項

App installation/資格情報、branch protection、実公開チャネル、Flux の push、PR 作成と CI 起動は実環境で確認する。
クラスタへの source 移管・復号・デプロイ・疎通も未確認。描画成功だけで実反映完了とは扱わない。
