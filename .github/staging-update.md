platform/main の公開済み成果物を Flux が検出し、digest を固定した更新です。
後続の更新候補もこの PR にまとめます。必要なタイミングで squash merge すると staging へ反映します。

CI、Chart と各 image の digest、アプリと Migrator の組み合わせを確認してください。
成果物の公開タイミングは異なるため、必要な成果物が揃っているとは限りません。
merge 後に staging の Ready と疎通を確認してください。
Flux が更新するブランチなので、手動修正や切り戻しは別ブランチの PR で行ってください。
