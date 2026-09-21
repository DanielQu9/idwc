# IdwC

**I don't write C.**

[English](README.md) · [嚴格語意規格](docs/strict-semantics.zh-TW.md) ·
[Stupid Mode](docs/stupid-mode.zh-TW.md) ·
[版本紀錄](CHANGELOG.md) · [發布流程](RELEASING.md)

*人生苦短，別再手動管理每一根指標。*

---

## 不想再寫 C 了嗎？

受夠了只是想做點小事，卻得先和指標搏鬥嗎？

沒有 `String`？只有 `char*` 和一點祈禱？

想串接兩個字串，卻不想順便重新思考人生？

懷念 Rust 的型別系統、ownership 和現代語法嗎？

**不。不。不。真的夠了。**

隆重介紹 **IdwC**——*I don't write C.*

這是一個革命性、突破性，而且完全沒有必要的方案。原本的問題大概只要
認真學 C 就能解決。

寫 Rust。得到 C。假裝一切都很好。

```bash
idwc main.rs -o main.c
gcc -std=c17 main.c -o main
./main
```

恭喜！你成功寫出了一個 C 程式，而且完全沒有寫 C。

*好吧，技術上不完全是。*

---

## 為什麼要做 IdwC？

因為我選到了一門 C 語言課。

因為我已經會 Rust。

因為我不想第幾千次寫下 `printf("%d\n", x)`。

也因為花好幾週打造轉譯器，只為了逃避幾行 C，顯然是我做過最合理的
工程決策。

**聰明工作，不要辛苦工作。**

*或者像這次一樣，用多很多的工作逃避一點點工作。*

---

## 功能

- 寫 Rust，產生 C17。
- 生成程式不需要 Rust runtime。
- 不必手動把作業翻譯成 C。
- 不必因為 `char*` 陷入存在主義危機。
- 完全沒有承諾要支援整個 Rust 語言。

IdwC 不支援完整 Rust。尤其是目前用於行輸入的限定 `String`，並不是一般
可成長的 Rust String，目前也不支援字串串接。看來要用 ownership model
取代 `strcat`，光靠一段氣勢十足的 README 開場還不夠。

實際支援的部分仍使用真正的 `syn` AST、白名單驗證、語意分析、typed IR
與 C codegen。在預設的嚴格模式中，檢查式算術、陣列邊界、輸入錯誤與
binary64 行為都是正式保證。不支援的語法會明確拒絕，不會靠猜、不會默默
忽略，也不會創造令人興奮的全新 C 未定義行為。

完整契約請參考[嚴格語意規格](docs/strict-semantics.zh-TW.md)。

## 里程碑

目前版本為 **v1.2.1**。

| 版本 | 目標 | 狀態 |
| --- | --- | --- |
| v0.1.0 | Hello World 端到端轉譯 | 已完成 |
| v0.2.0 | 變數、基本型別與運算式 | 已完成 |
| v0.3.0 | 條件判斷與迴圈 | 已完成 |
| v0.4.0 | 函式、輸出與有限型別化 stdin | 已完成 |
| v0.5.0 | 嚴格 `f64` 運算與輸入輸出 | 已完成 |
| v0.6.0 | 固定陣列、索引與邊界檢查 | 已完成 |
| v0.7.0 | 行輸入、token 解析與 `powi(2)` | 已完成 |
| v0.8.0 | 整數 range `for` 與核心子集補齊 | 已完成 |
| v0.9.0 | API、診斷、可攜性與發布強化 | 已完成 |
| v1.0.0 | 凍結嚴格 Rust 子集與穩定文件 | 已完成 |
| v1.1.0 | 可選的 Stupid Mode：寬鬆、可讀的 C | 已完成 |
| v1.1.1 | 完整 CLI 說明與可省略的 `-o` 輸出路徑 | 已完成 |
| v1.2.0 | 限定字串、固定容量 `Vec` 與 collection 輸出 | 已完成 |
| v1.2.1 | Stupid Mode 的字串與 Vec 輸出整理 | 已完成 |
| v1.3.0 | clap 管理 CLI 與依 locale 選擇中英文輸出 | 已規劃 |
| v1.4.0 | Collection 易用性與更多 collection 輸出 | 已規劃 |
| v1.5.0 | Bounded String 編輯 | 已規劃 |
| v1.6.0 | Eager 陣列與 Vec `.map` 翻譯 | 已規劃 |
| v1.7.0 | 短 iterator pipeline 與 reduction | 已規劃 |
| v1.8.0 | 限定的 pattern-based control flow | 已規劃 |

v1.2 系列支援綁定字串字面量的 `&str`、有容量上限的 owned `String`、
`idwc::io::read_line() -> String`，以及使用既有純量型別的固定容量 `Vec<T>`。
Vec 子集包含 `Vec::new()`、兩種 `vec!`、`push`、索引、`.len()`、賦值與
local move。`i32`、`usize`、`bool` 的 Vec 與固定陣列可用專用 `{:?}` 輸出，
並直接降低為 C 迴圈；`f64` collection 會先拒絕，直到能符合 Rust Debug
格式。`--string-capacity` 與 `--vec-capacity` 用來設定生成的 stack storage。
這不代表支援一般 Rust reference、heap allocation、slice、iterator，或完整
`String`、`Vec`、`Debug` API。看來「就用陣列啊」只要碰上 Rust 語意，也能
膨脹成一份設計文件。

### CLI 與語言規劃

v1.3.0 會以 [clap](https://github.com/clap-rs/clap) 取代手寫參數 parser，並
保留目前所有 flag、可省略的輸出路徑、容量驗證及原子檔案替換。參數關係、
重複選項、值、help 與 version 將交由 clap 管理。目前的標準函式庫 parser
證明第三方 dependency 並非絕對必要；採用 clap 的理由是避免每個新選項都
重複實作 parsing、usage 與錯誤回報。

CLI 會在產生 help 或錯誤前決定回應語言。`IDWC_LANG` 提供使用者與測試可重現
的明確 override；否則依序檢查 `LC_ALL`、`LC_MESSAGES`、`LANG`。正規化後以
`zh` 開頭的 locale 使用繁體中文，其餘、缺少、無效、`C` 或 `POSIX` locale
一律使用英文。Locale 偵測只影響 IdwC 訊息，不會改變生成程式的數字或 C
locale 行為。

本地化範圍包含 help、參數錯誤、檔案 I/O 錯誤、Stupid Mode warning 與轉譯
診斷。由於 clap 目前沒有完整的 runtime localization，IdwC 會使用
`try_get_matches_from` 加上一層小型雙語 renderer，而不讓 clap 直接以固定英文
退出。轉譯錯誤會攜帶穩定 diagnostic key 與結構化值，讓 CLI 不必解析既有
訊息即可選擇文字。Library API 不讀取 process locale，維持 deterministic；
現有 1.x diagnostic accessor 也會保持相容。

預定的 v1.4.0 collection 補強會以固定 `&str` 陣列、符合 Rust 的 `f64`
collection `{:?}` 輸出，以及對支援陣列與 Vec 的 by-value `for` 迴圈為目標。
這會直接涵蓋 `&str` 姓名陣列搭配 `f64` 成績陣列等常見的平行陣列程式。

接著 v1.5.0 會為 bounded String 加入一小組具容量檢查的編輯 API，先從
`.len()`、`.is_empty()`、`.clear()` 與 `.push_str()` 開始。

### 現代 Rust 語法降低計畫

v1.6.0 保留給 eager `.map` 翻譯。固定陣列會接受真正的 Rust 寫法
`values.map(|value| expression)`；Vec 則使用
`values.into_iter().map(|value| expression).collect()` 這類真正的 iterator
寫法，而不發明只有 IdwC 能理解的 `Vec::map`。兩者都會直接降低成 indexed C
`for` 迴圈與固定長度或固定容量的目的 collection。嚴格模式會保留 closure
內容中的檢查式運算，Stupid Mode 則直接生成易讀運算子。第一階段只接受
expression-only closure，並可捕捉目前已支援的 scalar binding。

v1.7.0 會在這套機制上加入短且能靜態理解的 pipeline，目標包含陣列與 Vec
的限定 `.filter`、`.enumerate`、`.zip`、`.fold`、`.sum`、`.any` 與 `.all`。
IdwC 會在可行時把安全的 pipeline 融合成一般迴圈，`.any` 與 `.all` 則使用
提早退出；不會建立一般 iterator runtime 或 heap intermediate collection。

v1.8.0 會補上讓這些 API 更好用的 control-flow sugar：小型 tuple destructuring，
以及針對定義明確的 scalar／collection 結果所提供的限定 `match`、`if let` 與
`while let`。一般 pattern、reference、作為儲存值的 closure 和完整
`Iterator` API 仍不在這份計畫內。Rust 負責看起來現代，C 負責老實跑迴圈。

這些項目目前只是計畫；每項功能完成並通過測試後，才會加入嚴格語意契約。

## 環境需求

- Rust 1.88 以上，用於編譯 IdwC。
- Clang 或 GCC，用於編譯生成的 C17。
- 在部分平台上，浮點程式需要連結 `-lm`。

## 安裝

使用測試過的 dependency lockfile，從 crates.io 安裝已發布的 CLI：

```bash
cargo install idwc --locked
```

Cargo 會自動下載、編譯 IdwC，並將 `idwc` 執行檔安裝到 Cargo 的 binary
目錄，通常是 `~/.cargo/bin`。


若要自行從 Git repository 下載並編譯：

```bash
git clone https://github.com/DanielQu9/idwc.git
cd idwc
cargo build --release --locked
./target/release/idwc --version
```

## 快速開始

使用已安裝的 CLI 轉譯並編譯 Hello World：

```bash
idwc examples/hello.rs -o /tmp/idwc-hello.c
clang -std=c17 /tmp/idwc-hello.c -o /tmp/idwc-hello
/tmp/idwc-hello
# Hello, World!
```

在原始碼 checkout 中，也能不安裝而透過 Cargo 執行相同轉譯：

```bash
cargo run -- examples/hello.rs -o /tmp/idwc-hello.c
```

使用 Stupid Mode 產生刻意寬鬆、適合人類閱讀的 C：

```bash
idwc --stupid examples/stupid_stdin.rs -o /tmp/idwc-stupid-stdin.c
# 短選項：idwc -s examples/stupid_stdin.rs -o /tmp/idwc-stupid-stdin.c
```

此命令會在 stderr 顯示提示，因為輸出刻意省略嚴格 runtime 檢查。完整規則
中，v1.2 collection 的簡單 Vec 寫入會保持直接，只有 expression 副作用需要
時才建立 temporary；只有後續未讀取的 Vec storage 或 length 才生成 `(void)`
warning suppression。完整規則請見 [Stupid Mode 規格](docs/stupid-mode.zh-TW.md)。

使用 `--help` 查看用法，使用 `--version` 或 `-V` 查看版本。輸入必須是
`.rs`，輸出必須是 `.c`。`-o` 可以省略：`idwc path/program.rs` 會寫入
`path/program.c`，加上 `-o` 則可指定其他位置。IdwC 會先完成轉譯與暫存檔
寫入，再以原子 rename 取代輸出，因此轉譯失敗或部分寫入失敗不會截斷既有
C 檔案。

bounded String 預設容納 4096 payload bytes，固定容量 Vec 預設容納 2048 個
elements。每次轉譯可用 `--string-capacity <bytes>` 與
`--vec-capacity <elements>` 調整，兩者皆須介於 1 到 65536。這些設定會改變
生成的 stack object 大小；容量很大或同時存在很多 collection 時會增加 stack
用量。

### 整數 range 範例

```rust
fn main() {
    let mut total = 0;
    for value in 1..5 {
        if value == 2 {
            continue;
        }
        total += value;
    }
    println!("total = {}", total);
}
```

轉譯並執行版本庫內的範例：

```bash
cargo run -- examples/ranges.rs -o /tmp/idwc-ranges.c
clang -std=c17 /tmp/idwc-ranges.c -o /tmp/idwc-ranges
/tmp/idwc-ranges
```

`for binding in start..end` 和 `start..=end` 接受型別相同的 `i32` 或 `usize`
bounds。上下限依序且各自只求值一次。迭代 binding 有自己的 scope，`break`
與 `continue` 依 Rust 行為運作，inclusive range 也能在整數最大值安全停止。

### 行輸入範例

```bash
cargo run -- examples/line_input.rs -o /tmp/idwc-line-input.c
clang -std=c17 /tmp/idwc-line-input.c -lm -o /tmp/idwc-line-input
printf '70 1.75\n' | /tmp/idwc-line-input
# Enter weight (kg) and height (m): BMI = 22.86, below 25 = true
```

其他範例涵蓋變數、控制流程、函式與型別化 stdin、浮點、固定陣列、字串及
固定容量 Vec。每個提交的 `examples/*.c` 都是由相對應 Rust 原始碼產生的
golden output，並由測試確認內容一致。

## 目前子集概覽

- 恰好一個 private `fn main()`，不可有參數或顯式回傳型別。
- private 輔助函式可使用 scalar 參數，以及 scalar 或 unit 回傳；支援向前
  呼叫與遞迴。
- `i32`、限定 `usize`、`f64`、`bool`、一維固定陣列、bounded String 與
  固定容量 scalar Vec。
- 初始化的 `let`／`let mut`、賦值、shadowing 與詞法區塊。
- 檢查式算術、比較、布林運算及短路求值。
- statement 形式的 `if`／`else if`／`else`、`while`、`loop` 和整數 range
  `for`，以及無 label 的 `break`／`continue`。
- `[T; N]`、陣列字面量、repeat 初始化、值複製、`.len()` 和經檢查的
  `usize` 索引。
- `T = i32 | usize | f64 | bool` 的 `Vec<T>`，支援 `Vec::new()`、
  `vec![...]`、`vec![value; N]`、`push`、`.len()`、檢查式索引、賦值與 move。
- `print!`／`println!`，包含無參數形式、依序 `{}` placeholder，以及 f64
  的 `{:.0}` 到 `{:.18}`，另可用限定 `{:?}` 輸出 `i32`、`usize`、`bool`
  陣列與 Vec。
- `idwc::io::read_i32()`、`read_f64()`、`read_line()` 和 `flush_stdout()`。
- 限定的 `String::new()` → `read_line(...).unwrap()` → `trim()`／
  `split_whitespace()` → `parse::<i32|f64>().unwrap()` 輸入流程。
- `idwc::io::read_line()` 可用較短寫法建立並填入一個 bounded `String`；它會
  保留換行，若一開始就是 EOF 則回傳空 String。
- 可綁定字串字面量的 `&str`，以及 bounded `String::new()`／
  `String::from(&str)`、賦值、move 檢查與 `{}` 輸出。
- `f64` 的 `powi(2)` 方法。

主要未支援項目包含未列出的 String／Vec 操作、字串串接、clone、slice、struct、enum、
cast、`f32`、其他整數型別、一般 iterator、closure、`match`、async、unsafe、
pointer、泛型、trait、任意 macro、完整 `std`，以及輸入程式的 Cargo 依賴。

精確語法、限制、求值順序、診斷及 C runtime 行為請見
[嚴格語意規格](docs/strict-semantics.zh-TW.md)。

## 架構

```text
Rust source
    → syn AST
    → 白名單驗證
    → 語意分析與 scope／型別檢查
    → typed custom IR
    → C17 code generation
```

預設的公開入口仍使用嚴格模式：

```rust
pub fn transpile(source: &str) -> Result<String, TranspileError>;
```

Library 使用者可呼叫 `transpile_with_options` 並指定
`TranspileMode::Stupid`；`transpile(source)` 永遠使用嚴格模式。

轉譯失敗會提供穩定的 `TranspileErrorKind` 分類（`Parse`、`Unsupported` 或
`Semantic`）、不含前綴的原始訊息，以及可選的一基準 `SourceLocation`。
`Display` 會包含分類與位置，適合直接作為 CLI 診斷。若錯誤屬於整份程式，
例如缺少 `main`，則可能沒有對應位置。

IdwC 1.x 的公開 Rust API，以及嚴格語意規格所記錄的接受行為，會遵守
Semantic Versioning。生成 C 仍以可讀為目標，但空白、helper 排列及內部
識別字名稱不屬於穩定的逐字文字 API。

`src/main.rs` 只處理 CLI 參數與檔案 I/O；翻譯規則保留在 library。
`src/runtime/` 內的 C 片段只在程式需要時嵌入，最終結果仍是單一 C 檔案。

## 開發與驗證

```bash
cargo fmt -- --check
cargo check --locked
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
```

端到端測試會分別編譯可信任 Rust 與生成的 C，提供相同輸入，再比較輸出
bytes 與退出狀態。整數及陣列失敗案例也會使用 UndefinedBehaviorSanitizer。
CI 會使用宣告的 Rust 1.88 MSRV，在 Linux 使用 Clang／GCC，並在 macOS
使用 Clang 執行測試。

## Stupid Mode

`-s`／`--stupid` 優先產生方便人類閱讀及修改的 C：能保留的原始名稱就
保留，可見 Unicode 直接出現在 `u8` 字串中，並以直接 C 運算子與標準函式
取代 IdwC runtime helper。它不產生額外 helper function 或 struct，也不加入
overflow、陣列邊界、輸入格式檢查或浮點環境初始化；Rust 原始碼本來定義的
函式仍會正常生成。

名字可以胡鬧，規格不能含糊。signed overflow、除以零、無效輸入、陣列越界、
原生浮點格式與 C 求值順序都不在此模式的保證內。Parser、AST 白名單、名稱
解析與型別檢查仍然保留。嚴格模式維持預設且行為不變；完整取捨請見
[Stupid Mode 規格](docs/stupid-mode.zh-TW.md)。

## 授權

IdwC 採用 [MIT License](LICENSE)。

如果你的 Rust 寫得太複雜，IdwC 會很有禮貌地拒絕翻譯。

大概吧。

---

## 專案哲學

> 為什麼要學 C，當你可以花三週寫一個編譯器來逃避它？

這個專案不是要取代 C。

也不是要超越 GCC。

更不是要重新發明系統程式設計的未來。

它只遵守一條很簡單的原則：

**I. Don't. Write. C.**

---

*IdwC——從 2026 年開始，把一份簡單作業變成完全沒有必要的編譯器專案。*
