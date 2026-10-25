# PLATFORM-20261024-01 の上流情報検証

## 結論

**再現性と供給元の検証を改善する対応は必要。ただし、改ざん・侵害・過去の drift が発生したとの断定はできない**。 Flux と CNPG は実在する公式リリースだが、対象リリースの GitHub API はいずれも `immutable: false`。外部 YAML の参照固定と、そこに含まれるコンテナイメージの digest 固定は別々に必要となる。現在の照合結果が一致することは、過去や将来の不変性を保証しない。[Flux API](https://api.github.com/repos/fluxcd/flux2/releases/tags/v2.9.6)、[CNPG API](https://api.github.com/repos/cloudnative-pg/cloudnative-pg/releases/tags/v1.30.1)、[Kubernetes のタグと digest](https://kubernetes.io/docs/concepts/containers/images/#image-names)

優先するのは、レビュー済み YAML のリポジトリ内保存、完全なイメージ digest の採用、更新時の検証手順である。署名・レビューの強制は staging の main 直接 push と整合させて段階導入する。今回の成果物は検証メモだけであり、設定やクラスタは変更していない。

影響度 High という判断は妥当であり、早期の計画対応が必要である。上流の配布物や公開経路を攻撃者が変更できることが前提で、公開 endpoint から直接悪用できる問題は今回確認していない。緊急停止を要する根拠は得られていないが、これは侵害がないことの証明ではない。

## 時点・証拠の範囲

- 取得時計は `2026-10-24 19:31:08–19:34:44 UTC`、すなわち 2026-10-25 04:31:08–04:34:44 JST。配布物の HTTP Date も同日の UTC 時刻で整合する。親調査の時計は 04:33 JST。直接取得した公開日時と時計に矛盾はなかった。検索結果の相対日時や古い一覧抜粋より、対象リリースの API と実ファイルを優先した。
- 上流確認は GitHub API、公式配布ファイル、公式文書・対象コミットのソースを使用。`gh` はすべて `mise exec -- gh ...` を sandbox 外で実行した。配布物はメモリ内取得し、SHA-256 を計算した。
- ローカル構成・管理設定は親の確定調査を統合したもの。対象 HEAD は `e9cbac544e0ebdba2a665ee8154457f57905c5db`。親は報告対象の3ファイルとの一致、および Kustomize v5.8.2 による controllers／operator／staging-db／production-db の build 成功を確認した。build 成功は live の適用・稼働確認を意味しない。
- 機密の読み取り、live クラスタ接続・変更、Cosign 等による暗号学的署名検証は実施していない。

## 上流リリースとコミットの照合

| 対象         | 取得結果                                                                                                                                       | 根拠・限界                                                                                                                                                                                                                                                                                                              |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Flux v2.9.6  | 公開済み、`draft: false`、`prerelease: false`、`immutable: false`。`published_at: 2026-10-01T09:56:48Z`（同日 18:56:48 JST）                   | [対象リリース API](https://api.github.com/repos/fluxcd/flux2/releases/tags/v2.9.6)。`target_commitish: main` は実際のタグの指すコミットを示す値として扱わない。                                                                                                                                                         |
| Flux のタグ  | annotated tag object `fe9f21f6ddcbbf5d5982453075221250a94076de` をたどると、commit は `b9c17924adb533d3617ec7de97a7b0f4acc07b39`。報告値と一致 | [タグ ref](https://api.github.com/repos/fluxcd/flux2/git/ref/tags/v2.9.6)、[tag object](https://api.github.com/repos/fluxcd/flux2/git/tags/fe9f21f6ddcbbf5d5982453075221250a94076de)。API の tag 署名表示は `verified: true`／`reason: valid`。配布 YAML の署名検証結果ではない。                                       |
| CNPG v1.30.1 | 公開済み、`draft: false`、`prerelease: false`、`immutable: false`。`published_at: 2026-09-23T13:35:42Z`（同日 22:35:42 JST）                   | [対象リリース API](https://api.github.com/repos/cloudnative-pg/cloudnative-pg/releases/tags/v1.30.1)。親取得の `target_commitish` も `main`。                                                                                                                                                                           |
| CNPG のタグ  | ref の object type は `commit`、値は `2a35abb4628f209d149825ef3c38011e0701ff2f`。報告値と一致                                                  | [タグ ref](https://api.github.com/repos/cloudnative-pg/cloudnative-pg/git/ref/tags/v1.30.1)。[commit API](https://api.github.com/repos/cloudnative-pg/cloudnative-pg/commits/2a35abb4628f209d149825ef3c38011e0701ff2f) の署名表示は `verified: true`／`reason: valid`。この ref 自体は annotated tag の署名を示さない。 |

GitHub の immutable release は、公開後の関連タグの移動と添付 asset の変更・削除を防ぐ機能である。今回の `false` はその機能による保護を確認できないという意味であり、変更の発生や、別のタグ保護が存在しないことを証明しない。 リリースの不変性と GHCR コンテナタグの不変性も別の対象である。機能の有効化は将来のリリースに適用される。[GitHub immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases)、[有効化の適用範囲](https://docs.github.com/en/code-security/how-tos/secure-your-supply-chain/establish-provenance-and-integrity/prevent-release-changes)

## 配布 YAML とイメージ参照

| 配布物                                                                                                                          | 今回の取得結果                                                                                                | 評価                                                                                                                                                                                                                                                                                                                               |
| ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Flux `install.yaml`](https://github.com/fluxcd/flux2/releases/download/v2.9.6/install.yaml)                                    | HTTP 200、341,661 bytes。計算 SHA-256 は `9c1fda7e401429531ed1478f67ba06b3edff6513b75da7ee47bde9f1c4d4251c`   | [release API](https://api.github.com/repos/fluxcd/flux2/releases/tags/v2.9.6) の asset digest と [checksums.txt](https://github.com/fluxcd/flux2/releases/download/v2.9.6/flux_2.9.6_checksums.txt) に一致。asset `updated_at` は `2026-10-01T09:56:44Z`。同じ配布元との整合確認であり、独立した真正性の証明や履歴の証明ではない。 |
| [CNPG タグ指定 raw manifest](https://raw.githubusercontent.com/cloudnative-pg/cloudnative-pg/v1.30.1/releases/cnpg-1.30.1.yaml) | HTTP 200、1,293,413 bytes。計算 SHA-256 は `37237f145d8138256ea25ae830f87759255665ff08f8d552fdd8224a5ec032fb` | [完全 commit 指定の同ファイル](https://raw.githubusercontent.com/cloudnative-pg/cloudnative-pg/2a35abb4628f209d149825ef3c38011e0701ff2f/releases/cnpg-1.30.1.yaml) も同じサイズ・SHA-256。release API の同名 asset digest とも一致。現在の一致であり、過去のタグ移動を否定しない。                                                 |

Flux 配布 YAML と親の描画結果で、次の7参照はすべて `ghcr.io/fluxcd/` 配下のタグ指定・`IfNotPresent` だった。[公式配布物](https://github.com/fluxcd/flux2/releases/download/v2.9.6/install.yaml)

```text
helm-controller:v1.6.5
image-automation-controller:v1.2.5
image-reflector-controller:v1.2.5
kustomize-controller:v1.9.6
notification-controller:v1.9.4
source-controller:v1.9.6
source-watcher:v2.2.4
```

CNPG Operator は `ghcr.io/cloudnative-pg/cloudnative-pg:1.30.1`・`Always`。これは DB 本体の PostgreSQL イメージとは別である。[対象 manifest](https://raw.githubusercontent.com/cloudnative-pg/cloudnative-pg/2a35abb4628f209d149825ef3c38011e0701ff2f/releases/cnpg-1.30.1.yaml)

Flux の release asset は生成物なので、ソースの tag commit 一致だけで `install.yaml` の内容まで固定したとはいえない。CNPG raw URL は ref 部分を完全 commit に置換できるが、**YAML を commit 固定しても内部のコンテナタグは固定されない**。[Flux 対象リリース workflow](https://github.com/fluxcd/flux2/blob/b9c17924adb533d3617ec7de97a7b0f4acc07b39/.github/workflows/release.yaml)、[CNPG commit 指定 manifest](https://raw.githubusercontent.com/cloudnative-pg/cloudnative-pg/2a35abb4628f209d149825ef3c38011e0701ff2f/releases/cnpg-1.30.1.yaml)

### PostgreSQL `18.6-system-trixie`

`ghcr.io/cloudnative-pg/postgresql:18.6-system-trixie` は `MM.mm-TYPE-OS` 形式の rolling tag に該当する。公式 README はこの形式を指定 minor version の最新イメージと定義し、週次再 build を説明する。PostgreSQL minor version が同じでも内容が変わり得るという指摘には根拠がある。一方、公式はこのタグを利便性と安定性の均衡として推奨しており、利用自体を誤りと断定するのは過大評価である。[公式 README: Image Tags / Key Features](https://github.com/cloudnative-pg/postgres-containers/blob/main/README.md#image-tags)

同 README の `MM.mm-TS-TYPE-OS` は build timestamp を含む識別用タグだが、日時付きタグというだけではレジストリによる上書き禁止を証明しない。内容固定には `ghcr.io/cloudnative-pg/postgresql:18.6-system-trixie@sha256:<完全な64桁>` のような digest 参照を使う。タグと digest の併記では pull に digest が使われる。今回は当該タグの現在の registry digest・実在確認を行っていないため、採用する digest 値は提示しない。[公式タグ仕様](https://github.com/cloudnative-pg/postgres-containers/blob/main/README.md#image-tags)、[Kubernetes の参照仕様](https://kubernetes.io/docs/concepts/containers/images/#image-names)

## 親調査の確定事実と影響条件

| 確定した設定・結果                                                                                                                                                                                                                                                                                                             | 意味と証拠の限界                                                                                                                                                                                                                                                    |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [controllers](../../infrastructure/base/controllers/kustomization.yaml) と [operator](../../infrastructure/base/operator/kustomization.yaml) が外部 YAML を参照。[対応 controllers Kustomization](../../clusters/shared/controllers.yaml) と [operator Kustomization](../../clusters/shared/operator.yaml) は `interval: 5m`   | 親の描画結果と今回の上流取得結果が一致。外部取得は下記の再 reconcile 条件に依存する。                                                                                                                                                                               |
| [DB base](../../infrastructure/base/database/cluster.yaml) は `18.6-system-trixie` のタグ指定。[staging](../../infrastructure/staging/database/kustomization.yaml) と [production](../../infrastructure/production/database/kustomization.yaml) が同じ base を使い、環境別の `imageName` 上書きなし                            | 両環境の desired state に同じ参照がある。live の採用イメージは未確認。                                                                                                                                                                                              |
| [production DB Kustomization](../../clusters/production/database.yaml) は `suspend: true`、[staging DB Kustomization](../../clusters/staging/database.yaml) は非停止                                                                                                                                                           | production DB の Flux reconcile は停止設定。ただし DB Pod の停止、CNPG Operator の停止、共有コントローラ経由の影響遮断を意味しない。                                                                                                                                |
| 親の描画で `cluster-reconciler` ClusterRoleBinding が kustomize-controller と helm-controller を `cluster-admin` にバインド                                                                                                                                                                                                    | 改変された desired state が到達した場合の影響範囲は大きい。侵害の証拠ではない。上流 [install.yaml](https://github.com/fluxcd/flux2/releases/download/v2.9.6/install.yaml) と親のローカル build が根拠。                                                             |
| [staging image automation](../../infrastructure/staging/image-automation/staging.yaml) は `git.push.branch: main`、commit `signingKey` なし。[shared](../../clusters/shared/source.yaml)／[staging](../../clusters/staging/source.yaml)／[production](../../clusters/production/source.yaml) の GitRepository は `verify` なし | 署名生成・検証の設定が存在しないという事実。全コミットが無署名だったことの証明ではない。                                                                                                                                                                            |
| main の実効 ruleset `Protect main` #25752458 は active。deletion、non_fast_forward、pull_request、`required_approving_review_count: 0`、`required_status_checks: []`。署名ルールなし。OrganizationAdmin と Integration id `5432448` が always bypass                                                                           | 親の [ruleset API 調査](https://api.github.com/repos/AnvilSaba/infrastructure/rulesets/25752458) による。App 名は未確認。「保護なし」は誤りだが、必須レビュー・必須チェック・署名の保証もない。App の main 直接 push を考慮せず一律強化すると自動更新と衝突し得る。 |

### remote bases と imagePullPolicy の条件

Flux 公式資料は remote base を reconcile ごとに取得し、ローカル Git revision が同じでも外部内容の変化がクラスタに反映され得ると明記する。今回の5分設定はその機会を作るが、5分以内の適用保証ではない。controller の稼働、非 suspend、source artifact の取得、依存先の準備、ネットワーク、build・検証・apply の成功等が必要で、interval に jitter が入る場合もある。source の Git 更新時だけが再 build の契機だという解釈は誤り。[Flux security best practices](https://fluxcd.io/flux/security/best-practices/)、[interval / suspend](https://fluxcd.io/flux/components/kustomize/kustomizations/#interval)

`IfNotPresent` はノードに存在しないイメージを pull する。したがって Flux の同一タグでも、キャッシュがあるノードと新しいノードで異なる内容を使う可能性が残る。`Always` はコンテナ起動時にレジストリへ照会して参照を解決し、既存 layer を再利用する。CNPG Operator のタグ更新が稼働中コンテナを即時更新するわけではなく、5分ごとの Flux reconcile が毎回イメージを pull するわけでもない。これらの方針は内容固定や署名検証の代わりにはならない。[Kubernetes image pull policy](https://kubernetes.io/docs/concepts/containers/images/#image-pull-policy)

## 署名・attestation・digest 固定の有効性と限界

- Flux: release API に checksum の `.sig`／`.pem` と `provenance.intoto.jsonl` が存在する。対象 workflow は署名・provenance の生成を定義する。しかしファイルの存在、GitHub の commit/tag の Verified 表示、今回の SHA-256 一致は、それぞれ別の証拠である。今回は署名・provenance の暗号学的検証を実行しておらず、`install.yaml` の provenance subject と信頼する builder の一致を検証済みとはしない。[release API](https://api.github.com/repos/fluxcd/flux2/releases/tags/v2.9.6)、[対象 workflow](https://github.com/fluxcd/flux2/blob/b9c17924adb533d3617ec7de97a7b0f4acc07b39/.github/workflows/release.yaml)、[対象署名設定](https://github.com/fluxcd/flux2/blob/b9c17924adb533d3617ec7de97a7b0f4acc07b39/.goreleaser.yml)
- CNPG: release API に manifest、checksums、`.sigstore.json`、`multiple.intoto.jsonl` があり、対象版の security 文書は Cosign と provenance の検証方法を示す。これも検証可能性の根拠であり、今回の検証成功を意味しない。[release API](https://api.github.com/repos/cloudnative-pg/cloudnative-pg/releases/tags/v1.30.1)、[対象版 security 文書](https://github.com/cloudnative-pg/cloudnative-pg/blob/2a35abb4628f209d149825ef3c38011e0701ff2f/docs/src/security.md#image-signatures)
- PostgreSQL system イメージの注意: CNPG security 文書は operand の署名を一般的に説明する一方、イメージの配布元 README は署名対象を `minimal`／`standard` と記述する。対象に近い後者を踏まえ、`system` の当該 digest に有効な署名があるとは断定しない。記載の限定だけで「署名がない」とも断定できない。[配布元 Image Signatures](https://github.com/cloudnative-pg/postgres-containers/blob/main/README.md#image-signatures)
- 信頼する OIDC issuer、repository／workflow identity、subject digest 等を照合すれば出所・ビルド経路の確認に使える。ただし署名・attestation は安全性や脆弱性不存在を証明せず、署名を生成しただけでは消費側の防御にならない。[GitHub artifact attestations](https://docs.github.com/en/actions/concepts/security/artifact-attestations)
- digest 固定は同じタグの再解決による内容変更を防ぐ。承認時点ですでに不正な内容を選んだ場合、配布停止・削除、将来の脆弱性発見には対応できない。更新を止めれば修正版も自動採用されないため、検証・承認を伴う更新運用と組み合わせる。multi-architecture image では index と各 platform manifest の digest を区別して採用する必要があり、今回はその値も未取得。[Kubernetes の digest 固定と image index](https://kubernetes.io/docs/concepts/containers/images/)

## 報告に対する評価

| 報告の内容                                                          | 判定                                                                                                                                                                                                                                                                                           |
| ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 外部 manifest と内部画像が可変参照で、侵害時の影響が大きい          | 妥当。ローカル設定、上流 manifest、cluster-admin のバインドを確認した。                                                                                                                                                                                                                        |
| 両製品は正式リリースで、記載の tag commit を参照している            | 確認できた。現在の一致は将来の不変性を保証しない。                                                                                                                                                                                                                                             |
| 再起動・再 pull 等によって差し替えが反映され得る                    | 妥当。ただし Flux は IfNotPresent、CNPG Operator は Always で、反映条件が異なる。manifest の再取得と画像の再 pull も別の経路である。                                                                                                                                                           |
| 新しい既知の重大脆弱性や現在の侵害・不審な drift は確認されていない | 今回は裏付けを完了していない。脆弱性情報の網羅照合、live／registry／過去の履歴確認は未実施。短縮 digest から一致を判定できない。                                                                                                                                                               |
| digest 不一致時に reconciliation と image pull を停止する           | 一律の運用は推奨しない。PostgreSQL の rolling tag は正規の週次再 build でも変化する。承認済み digest との差を検知して出所と変更理由を確認し、不正が疑われる対象を限定して隔離する。Flux の suspend だけでは kubelet の pull や CNPG の Pod 管理は停止しない。pull の一律停止は復旧を妨げ得る。 |

## 具体的な優先対応

1. 優先度高：外部 YAML の内容をレビュー可能な形で固定する。 最小案は今回照合した完全 SHA-256 と取得元を記録して YAML をリポジトリ内へ保存し、差分レビューで更新すること。CNPG の完全 commit URL は代替になるが外部取得依存は残る。Flux release の `immutable: false` を「上流が不変性を保証する」と扱わない。既存の remote base を解消してから `--no-remote-bases=true` を導入する。[Flux の推奨](https://fluxcd.io/flux/security/best-practices/)
2. 優先度高：Flux 7画像、CNPG Operator、PostgreSQL を完全 digest へ固定する。 YAML 保存だけでは内部タグの可変性は残る。採用時に完全 digest・platform・取得元を確認し、可能な対象は署名／provenance の実検証も記録する。今回のメモにない digest や報告の略記値を補完して使わない。production DB は再開前の確認事項に含める。[Kubernetes の内容固定](https://kubernetes.io/docs/concepts/containers/images/#image-pull-policy)
3. 優先度中：更新を PR に集約し、CI を必須にして App の bypass を縮小する。後続の方針として、staging の更新候補は `automation/staging` の1つの PR にまとめ、必要なタイミングで squash merge する。App id `5432448` の名前は AnvilSaba GitOps Update。PR 作成権限と認証を用意し、staging の新しい更新経路を確認してから App の main bypass を削除する。署名必須化・automation の署名生成・source の署名検証は保留とする。この調査時点では管理設定を変更していない。現在の導入手順は [Flux の更新 PR 設定](../flux.md#更新-pr-と-main-の保護)を参照する。

未解決なのは実際の稼働 digest と desired state の一致、過去のタグ／asset／registry drift、各対象の署名・attestation 実検証、App の特定である。これらを未確認のまま、再現性改善の必要性と、侵害発生の有無に関する判断を区別する。
