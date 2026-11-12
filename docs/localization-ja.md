# Japanese interface (Progate fork)

The interface shows in Japanese when the process environment asks for it:

| Variable | Effect |
| --- | --- |
| `DECKCRAFT_LANGUAGE` | Wins over the locale: `ja` (or `ja_JP.UTF-8`) for Japanese, `en` for English |
| `LC_ALL`, then `LC_MESSAGES`, then `LANG` | The first one that is set and not empty, as POSIX picks the messages locale. `ja_JP.UTF-8`, `ja_JP`, `ja` → Japanese; `C`, `POSIX` and languages without a catalog → English |

English stays the default and the lookup key: command ids, document text, file names, the
control channel, the CLI and MCP never see translated text (`crates/ui-egui/src/i18n/mod.rs`).
The catalog is `crates/ui-egui/src/i18n/ja.tsv` in WordCraft's format (`context<TAB>English<TAB>日本語`);
a string without an entry shows in English. Placeholder prompts on the slides (`タイトルを入力`)
come from the same catalog through `deckcraft_render::set_prompt_text`; they are never written
to files. Layout names, theme names and other names stored in a presentation show translated
when the catalog has the exact name, and are saved unchanged.

## Why the terms follow Microsoft PowerPoint

The fork is used to teach PowerPoint for the MOS PowerPoint (Microsoft 365) exam, whose task
wording names ribbon tabs, buttons and dialogs as Japanese PowerPoint shows them. Learners must
find the same names here, so labels use **PowerPoint (Microsoft 365) Japanese feature and menu
names**, written with Microsoft's spacing between katakana words (`スライド ショー`,
`アクセシビリティ チェック`, `スライド マスター`). Descriptions, messages and tooltips are our own
wording. Upstream DeckCraft's clean-room rule allows feature and menu names; nothing else is
copied.

## Terms (main ones; the full list is `ja.tsv`)

### Tabs

| English | 日本語 |
| --- | --- |
| File | ファイル |
| Home | ホーム |
| Insert | 挿入 |
| Draw | 描画 |
| Design | デザイン |
| Transitions | 画面切り替え |
| Animations | アニメーション |
| Slide Show | スライド ショー |
| Record | 記録 |
| Review | 校閲 |
| View | 表示 |
| Shape Format | 図形の書式 |
| Picture Format | 図の形式 |
| Table Design / Layout | テーブル デザイン / レイアウト |
| Chart Design | グラフのデザイン |
| SmartArt Design | SmartArt のデザイン |
| Playback | 再生 |
| Slide Master / Handout Master / Notes Master | スライド マスター / 配布資料マスター / ノート マスター |

### Ribbon and menus

| English | 日本語 |
| --- | --- |
| New Slide | 新しいスライド |
| Layout | レイアウト |
| Reset | リセット |
| Section | セクション |
| Arrange | 配置 |
| Align | 配置 |
| Align Left / Align Center / Align Right | 左揃え / 左右中央揃え / 右揃え |
| Align Top / Align Middle / Align Bottom | 上揃え / 上下中央揃え / 下揃え |
| Distribute Horizontally / Vertically | 左右に整列 / 上下に整列 |
| Group / Ungroup / Regroup | グループ化 / グループ解除 / 再グループ化 |
| Bring to Front / Send to Back | 最前面へ移動 / 最背面へ移動 |
| Bring Forward / Send Backward | 前面へ移動 / 背面へ移動 |
| Selection Pane | 選択ウィンドウ |
| Format Shape | 図形の書式設定 |
| Shape Fill / Shape Outline / Shape Effects | 図形の塗りつぶし / 図形の枠線 / 図形の効果 |
| Alt Text | 代替テキスト |
| Mark as decorative | 装飾用にする |
| Header & Footer | ヘッダーとフッター |
| Slide Number / Date & Time | スライド番号 / 日付と時刻 |
| Text Box / Shapes / Icons / WordArt | テキスト ボックス / 図形 / アイコン / ワードアート |
| Table / Chart / SmartArt | 表 / グラフ / SmartArt |
| Video / Audio | ビデオ / オーディオ |
| Slide Size / Format Background | スライドのサイズ / 背景の書式設定 |
| Effect Options / Apply To All | 効果のオプション / すべてに適用 |
| Add Animation / Animation Pane / Animation Painter | アニメーションの追加 / アニメーション ウィンドウ / アニメーションのコピー/貼り付け |
| On Click / With Previous / After Previous | クリック時 / 直前の動作と同時 / 直前の動作の後 |
| Duration / Delay | 期間 / 遅延 |
| From Beginning / From Current Slide | 最初から / 現在のスライドから |
| Custom Slide Show | 目的別スライド ショー |
| Set Up Slide Show | スライド ショーの設定 |
| Hide Slide | 非表示スライドに設定 |
| Rehearse Timings | リハーサル |
| Presenter View | 発表者ツール |
| Check Accessibility / Accessibility Checker | アクセシビリティ チェック |
| New Comment | 新しいコメント |
| Normal / Outline View / Slide Sorter / Notes Page / Reading View | 標準 / アウトライン表示 / スライド一覧 / ノート / 閲覧表示 |
| Ruler / Gridlines / Guides | ルーラー / グリッド線 / ガイド |
| Add Chart Element / Quick Layout | グラフ要素を追加 / クイック レイアウト |
| Axis Titles / Chart Title / Data Labels / Gridlines / Legend | 軸ラベル / グラフ タイトル / データ ラベル / 目盛線 / 凡例 |
| Start Automatically, Play Across Slides, Loop until Stopped | 自動的に開始、スライド切り替え後も再生、停止するまで繰り返す |
| Hide During Show, Rewind after Playing, Play Full Screen, Volume | 再生中のみ表示、再生が終了したら巻き戻す、全画面再生、音量 |

### File (Backstage)

| English | 日本語 |
| --- | --- |
| Home / New / Open / Info | ホーム / 新規 / 開く / 情報 |
| Save / Save As / Print / Export / Close | 上書き保存 / 名前を付けて保存 / 印刷 / エクスポート / 閉じる |
| Account / Options | アカウント / オプション |
| Check for Issues › Inspect Document | 問題のチェック › ドキュメント検査 |
| Document Inspector: Comments, Document Properties and Personal Information, Hidden Slides, Off-Slide Content, Presentation Notes | ドキュメント検査: コメント、ドキュメントのプロパティと個人情報、非表示のスライド、スライド外のコンテンツ、プレゼンテーション ノート |
| Remove All / Inspect / Reinspect | すべて削除 / 検査 / 再検査 |
| Protect Presentation › Mark as Final | プレゼンテーションの保護 › 最終版にする |
| Edit Anyway | 編集する |
| Tools › General Options › Password to modify | ツール › 全般オプション › 書き込みパスワード |
| Read Only | 読み取り専用 |

### Layouts and placeholder prompts

| English | 日本語 |
| --- | --- |
| Title Slide | タイトル スライド |
| Title and Content | タイトルとコンテンツ |
| Section Header | セクション見出し |
| Two Content | 2 つのコンテンツ |
| Comparison | 比較 |
| Title Only | タイトルのみ |
| Blank | 白紙 |
| Content with Caption | タイトル付きのコンテンツ |
| Picture with Caption | タイトル付きの図 |
| Title and Vertical Text | タイトルと縦書きテキスト |
| Vertical Title and Text | 縦書きタイトルと縦書きテキスト |
| Click to add title / subtitle / text | タイトルを入力 / サブタイトルを入力 / テキストを入力 |
| Click to add notes | ノートを入力 |

### Effects (context `effect`)

Transitions: なし、変形、フェード、プッシュ、ワイプ、スプリット、出現、カット、ランダム ストライプ、図形、
アンカバー、カバー、フラッシュ、変更、ドレープ、カーテン、風、大理石、破砕、クラッシュ、ピール オフ、
ページ カール、飛行機、折り紙、ディゾルブ、チェッカーボード、ブラインド、時計回り、波紋、ハチの巣、
グリッター、ボルテックス、シュレッド、スイッチ、フリップ、ギャラリー、キューブ、扉、ボックス、コーム、
ズーム、ランダム、パン、観覧車、コンベヤー、回転、ウィンドウ、オービット、フライスルー
(categories 弱 / はなやか / ダイナミック コンテンツ).

Animations: アピール、フェード、スライドイン、フロートイン、スプリット、ワイプ、図形、ホイール、
ランダム ストライプ、拡大して回転、ズーム、ターン、バウンド (開始); パルス、カラー パルス、シーソー、スピン、
拡大/収縮、… (強調); クリア、フェード、スライドアウト、フロートアウト、… (終了); 直線、アーチ、ターン、
ループ、ユーザー設定パス (アニメーションの軌跡).

Effect Options: 左から、右から、上から、下から、左上から…、横、縦、イン、アウト、黒いスクリーンから、スムーズ.

## Contexts

The same English means different things in a few places, so `ja.tsv` has a context column:

- `effect`: effect names and Effect Options (`Cut` is カット here, 切り取り on Home; `Horizontal`
  is 横 here, 横書き as a text direction).
- `chart`: Add Chart Element's menus (`Gridlines` is 目盛線 there, グリッド線 on View).
- `ribbon`: a ribbon button label that breaks over two lines differently from the same text elsewhere.

## Adding or changing a term

Edit `ja.tsv` (one line per string; `\n` for a line break in a ribbon label). The tests in
`crates/ui-egui/src/i18n/tests.rs` check that the catalog parses without errors and pin the main
PowerPoint terms.
