# production 更新 PR

platform は公開を所有し、infrastructure は配置の選択と検証を所有する。
実際の source 移管前にこの workflow の PR を merge しない。停止解除、DB import/切り替え、
新版 Chart/image の導入を source 移管と同時に行わない。

## 手動指定

infrastructure の **Production update PR** を main から実行する。
`bot`、`mc_link_server`、`public_api`、`chart` に各 component の公開済み `component/vX.Y.Z` を指定する。
空欄は現在の配置を維持する。複数 component は一つの PR にまとめる。
必要なら `migrator_revision` に公開済み `sha-<完全 SHA>` image に対応する完全 SHA を明示する。

初回の digest 化は、digest 対応済み Chart の release を明示し、必要な application release を選択する。
旧 Chart に revision annotation がなく組み合わせを検証できない場合は拒否する。
未指定アプリの参照は維持するため、残る tag 参照も別の明示的な更新で digest 化する。
旧タグ・実際の稼働内容・公開済み成果物が一致しない場合は、それを直さずに PR を作成しない。

検証処理は platform と同じ Cargo script 形式を使う。
入力ファイルの例:

```json
{
  "releases": {
    "bot": "bot/v4.0.1",
    "public-api": "public-api/v1.0.1",
    "chart": "chart/v1.0.1"
  },
  "migrator_revision": null,
  "automatic": false
}
```

ローカルで差分を確認するときは main とタグを取得した platform checkout を渡す。
`mise exec -- cargo -Zscript scripts/update.rs production <入力.json> <platform checkout>`。
このコマンドはファイルだけを更新し、PR 作成やクラスタ操作は行わない。
`mise run fmt`、`mise run fmt:check`、`mise run check`、`mise run test` で確認する。

## 整合性と切り戻し

release tag の component と Git commit を照合し、固定した GHCR 公開先から manifest/config を取得する。
manifest digest、OCI revision、release/SHA tag の対応を検証する。Chart の archive も digest で検証して
Helm lint/template を実行し、image の digest が実際の manifest に反映されることを確認する。

選択アプリの revision と現在の Migrator のうち、履歴上包含する新しい revision の公開済み Migrator を選ぶ。
別 revision を明示することもできるが、現在の Migrator より古くできない。
選択・据え置きアプリのすべての revision を Migrator が履歴に含むことと、必須 migration の SQL が
一致することを検証する。DB の意味的な後方互換性は、この検証だけでは証明できない。
PR のレビューでは expand-contract と既存アプリ・切り戻し対象の互換性を確認する。

アプリ切り戻しは古い release tag を手動指定する。Migrator は維持するか前進する。
自動通知では現在の配置より古い revision を提案せず、重複は変更なしとする。

## release 後の任意通知

platform の Release で `deploy` を有効にした場合だけ、必要な image と Migrator または Chart の
公開成功後に `platform-release` を送る。既定は無効であり、失敗・dry-run は通知しない。
単独の既存 release は platform の **Production update PR** からも依頼できる。
受信側は通知された release/revision/digest と実際の公開物を再照合する。

更新直前に最新 main を確認し、差分の計算中に main が変わったら再検証する。
検証後の配置内容から automation branch を決め、同じ配置なら自動通知・手動指定とも同じ PR を使う。
branch 更新には force-with-lease を使い、競合は再試行する。PR の同時作成が競合した場合も既存 PR を再取得して更新する。
main への直接 push や自動 merge は行わない。作成した PR の CI、差分、記録した artifacts.json を人が確認する。
権限・App・GHCR 認証は [staging の設定](staging.md#用途別の認証)に従う。

## 未確認事項

App installation、資格情報、保護ブランチの許可、実 package/tag の存在、実通知と PR CI の起動は
実際の GitHub Actions で確認が必要。クラスタの source 切り替え・復号・Flux/HelmRelease・疎通も未確認。
これらをローカル描画の成功で完了扱いにしない。source 引き継ぎは Issue #6 で行う。
