# IdwC 嚴格語意規格

[English](strict-semantics.md) · [返回 README](../README.zh-TW.md) ·
[Stupid Mode](stupid-mode.zh-TW.md)

本文定義 IdwC v1.2.0 預設嚴格模式的翻譯契約。未列出的內容一律拒絕。
生成程式應在不依賴 C 未定義行為的前提下，保留所接受 Rust 子集的行為。
可選的 `-s`／`--stupid` 生成器另有刻意放寬的[規格](stupid-mode.zh-TW.md)，
不會改變嚴格模式行為。

## 程式與函式

- 檔案頂層只能包含函式，且恰好有一個 private `fn main()`。
- `main` 不接受參數、修飾詞、屬性、泛型或顯式回傳型別。
- 其他 private 函式可使用 `i32`、`usize`、`f64`、`bool` scalar 參數，並
  回傳 scalar 或 unit；參數可標記 `mut`。
- 支援向前呼叫及遞迴；不支援呼叫 `main`、間接呼叫、任意 method 或泛型。
- 支援 `return;`、`return ();`、typed `return value;` 與函式尾端 scalar
  expression。保守分析要求每個非 unit 函式都有保證回傳路徑。
- 參數以值複製；每個函式都有獨立的詞法 scope。

## Binding 與型別

- Binding 使用已初始化的 `let` 或 `let mut`，pattern 限定單一 ASCII 名稱。
- Scalar 型別為 `i32`、限定 `usize`、`f64` 與 `bool`。
- 無後綴整數預設為 `i32`，只有明確的 `usize` context 會推導成 `usize`；
  支援 `i32` 與 `usize` 後綴。
- 支援十進位、十六進位、八進位和二進位整數字面量。可用
  `-2147483648` 表示最小值，但不支援 `i32::MIN` 等 associated path。
- 十進位及科學記號浮點字面量預設為 `f64`，可有 `f64` 後綴和底線；拒絕
  `f32` 與非十進位浮點。
- 支援同 scope 及巢狀 shadowing。未定義或超出 scope 的名稱、不可變賦值、
  型別不符皆為錯誤。
- 下文定義限定的 `&str`、owned `String` 與固定容量 scalar `Vec<T>`；這不
  代表支援一般 reference 或 collection。

## 運算式與求值

- 相同數字型別支援 `+`、`-`、`*`、`/`、`<`、`<=`、`>`、`>=`；`i32`
  與 `usize` 另外支援 `%`。
- `==`、`!=` 支援相同 scalar 型別；混合數字型別會被拒絕。
- 一元 `-` 支援 `i32`、`f64`；`!` 支援 `bool`。
- `&&`、`||` 短路求值。其他 binary operands 和函式引數以生成的 temporary
  保留由左至右的求值順序。
- 可變 binding 支援一般賦值及算術複合賦值；賦值只能作為 statement。
- 唯一支援的數學 method 是 `f64::powi(2)`；receiver 只求值一次，並執行
  一次 binary64 乘法。
- Scalar expression statement 可以丟棄結果，但仍執行檢查與副作用。不支援
  產生值的 block。

## 控制流程

- Statement 形式的 `if`／`else if`／`else` 要求 `bool` condition 和 unit
  branch，只有被選擇的 branch 會執行。
- 支援無 label 的 `while` 與 `loop`。`while` condition 在每次迭代前重新
  求值，包含 `continue` 之後。
- 無 label 的 `break`、`continue` 作用於最內層 loop，且不能攜帶值。
- Range loop 支援 `for binding in start..end` 和
  `for binding in start..=end`，bounds 必須同為 `i32` 或同為 `usize`。
- Range bounds 在開始迭代前，由 start 至 end 依序且各自只求值一次。
- Range binding 有新的詞法 scope，每次迭代都由獨立 iterator value 重新
  建立。即使使用 `for mut i` 並在 body 修改 `i`，也不影響下一個迭代值。
- 空 range 與反向 range 不執行。Inclusive range 不會遞增超過 endpoint，
  包含 `i32::MAX` 和 `usize::MAX`；`break`／`continue` 維持 Rust 行為。

拒絕 `for` 之外的 Range 值、open-ended range、非整數 range、label、一般
iterator、`.rev()`、`if let`、`while let`、`match` 和產生值的控制流程。

## 固定陣列

- 本地一維 `[T; N]` 支援 `T = i32`、`usize`、`f64` 或 `bool`。
- `N` 必須是 0 到 4096 的整數字面量。
- 支援陣列字面量、`[value; N]`、整體複製／賦值、`.len()`，以及命名
  binding 的元素讀寫。
- 索引必須是 `usize`，只求值一次，並在接觸 C storage 以前檢查。
- 零長度陣列在 C 端使用一個不可存取的實體元素，邏輯長度仍為零。
- 越界會 flush stdout、向 stderr 寫入 `idwc: array index out of bounds`，
  並以狀態 101 結束。
- 拒絕巢狀陣列、slice、reference、陣列參數／回傳、iterator method、陣列
  相等比較，以及索引臨時陣列 expression。

## 固定容量 Vec

- 本地 `Vec<T>` 支援 `T = i32`、`usize`、`f64` 或 `bool`。不支援 Vec
  參數／回傳、巢狀 collection、reference、slice 或任意元素型別。
- `Vec::new()` 與空 `vec![]` 必須有明確 `Vec<T>` 註記。非空 `vec![...]`
  會推導一種支援的 scalar 元素型別，混合型別會被拒絕。`vec![value; N]`
  只求值一次 `value`，`N` 必須是 0 到 65536 的無後綴整數字面量。
- Storage 是固定 C array 加 logical length。預設容量為 2048 elements，可用
  `--vec-capacity` 或 `TranspileOptions::with_vec_capacity` 逐次轉譯設定；不
  使用 heap allocation，也不會動態擴容。
- 可變 binding 支援 `push`、整體 Vec 賦值及元素賦值；支援 `.len()` 與命名
  binding 的 `usize` 索引。Vec 索引的複合賦值目前不接受。
- Vec 在 local binding 間採 move；讀取 moved value 會被拒絕。對可變且已
  moved 的 binding 賦予新 Vec 後即可再次使用。
- 建立或 `push` 超出設定容量時，會 flush stdout、向 stderr 寫入
  `idwc: Vec capacity exceeded`，並以狀態 101 結束；寫入 C storage 前會先
  檢查容量。
- 索引只求值一次；越界時在存取 C storage 前 flush stdout、向 stderr 寫入
  `idwc: Vec index out of bounds`，並以狀態 101 結束。
- 不支援 clone、相等比較、`pop`、`insert`、`remove`、`clear`、容量 method、
  iterator 與其他 Vec API。

## 輸出

- 支援無參數的 `print!()` 和 `println!()`。
- 其他形式必須先提供字串字面量，再依序提供引數。
- `{}` 接受 `i32`、`usize`、`f64`、`bool`，以及下文定義的 `&str`、
  `String`；`{:.0}` 到 `{:.18}` 只接受 `f64`；`{{`、`}}` 產生 literal brace。
- `{:?}` 接受元素為 `i32`、`usize` 或 `bool` 的命名一維固定陣列與 Vec，
  透過生成的 C 迴圈寫出 Rust 形式的方括號與 `, ` 分隔。`f64` collection
  因尚未實作 Debug 的指數與小數點規則而明確拒絕。
- 格式化訊息的所有引數會由左至右求值完畢，再開始寫出文字。
- 純文字 `println!` 使用 `puts`，純文字 `print!` 使用 `fputs`。
- 輸入文字不會成為 C format string。拒絕內嵌 NUL；UTF-8、控制 bytes 及
  問號會安全跳脫為 C17 字串。
- 拒絕 captured、numbered、named、alternate／pretty debug、width 和其他格式。
- 除了下述明確 flush 契約，不保證 stdout I/O 失敗與 Rust 完全一致。

## 型別化 token 輸入

`idwc::io::read_i32()` 與 `idwc::io::read_f64()` 讀取空白分隔 token；
`idwc::io::flush_stdout()` 用於互動提示。

- 空白包含 ASCII space、tab、LF、CR、vertical tab、form feed。
- Token 最多 128 bytes，包含 sign。
- `read_i32` 接受 `[+-]?[0-9]+` 並檢查完整 i32 範圍。
- `read_f64` 接受十進位／科學記號，以及恰好 `NaN`、`inf`、`+inf`、
  `-inf`。
- 在此 API 中，float overflow 至 infinity 或非零 underflow 至零屬於 range
  error；可表示 subnormal 與 signed zero 合法。
- Integer／float read 可交錯。有效 token 在 EOF 結束時成功，下一次 read
  才回報 EOF。

失敗會 flush stdout、寫出以下診斷，並以狀態 101 結束：

| 條件 | 診斷 |
| --- | --- |
| Token 前遇到 EOF | `idwc: unexpected EOF` |
| stdin 錯誤 | `idwc: stdin I/O error` |
| 無效整數 | `idwc: invalid integer` |
| 超出 i32 | `idwc: integer out of range` |
| 無效浮點 | `idwc: invalid float` |
| Float overflow 或非零 underflow 至零 | `idwc: float out of range` |
| 超過 128 token bytes | `idwc: input token too long` |
| 顯式 stdout flush 失敗 | `idwc: stdout flush error` |

## 限定字串值

- 字串字面量使用限定的 `&str` 型別，可用 `{}` 輸出，也可綁定至推導或明確
  註記的 `&str`；允許複製這類 binding。這不代表支援一般 reference、
  lifetime、slice 或 borrowing。
- Owned value 支援 `String::new()`、`String::from(&str)`、
  `idwc::io::read_line()`、可變賦值、local binding 間的 move 與 `{}` 輸出；
  函式仍不能接受或回傳字串型別。
- Strict C 同時保存 bytes 與明確長度，因此 `String::from` 和輸出會保留內嵌
  NUL byte 與 UTF-8。
- owned String move 後再次使用會被拒絕。對可變且已 moved 的 binding 賦予
  新值後即可再次使用；仍被 token collection 借用的 String 不能 move 或
  賦值。
- `String::from` 的來源超過設定的 payload 容量時，會輸出
  `idwc: String capacity exceeded` 並以狀態 101 結束。

一般串接、修改 method、clone、相等比較、字串索引、函式參數／回傳與任意
`&str` expression 仍不在子集內。

## 限定行輸入

行輸入接受以下本地模式：

```rust
let mut input = String::new();
std::io::stdin().read_line(&mut input).unwrap();
let values: Vec<&str> = input.split_whitespace().collect();
let number = values[0].parse::<f64>().unwrap();
```

v1.2.0 的便利介面會建立新的 bounded String，並以相同驗證規則讀取一次：

```rust
let input = idwc::io::read_line();
let number = input.trim().parse::<f64>().unwrap();
```

- 生成的 String 預設有 4096 bytes stack storage。`read_line` 採附加模式並
  保留 newline；若一開始即 EOF，成功且不修改內容。
- `idwc::io::read_line()` 從空 String 開始，因此空 EOF 會回傳空 String；此
  函式不接受引數。
- 累積內容不可超過設定的 String 容量，每次 read 後都必須是有效 UTF-8。
- `trim()`、`split_whitespace()` 使用與 Rust 相容的 Unicode White_Space。
- 收集的 `Vec<&str>` 預設最多包含 2048 個借用來源 buffer 的 stack span，
  不配置或複製 token 文字。
- 同一 String 所產生的 token collection 仍在 lexical scope 時，保守拒絕
  再次 `read_line`。
- 支援 `input.trim().parse::<i32|f64>().unwrap()` 和
  `tokens[index].parse::<i32|f64>().unwrap()`；token index 只求值一次並檢查
  邊界。Collection 也支援 `.len()`。
- String `parse::<f64>()` 遵循 Rust range result：overflow 變成 signed
  infinity，非零 underflow 可變成 signed zero；這與 typed-token
  `read_f64` 的 range 規則不同。

行輸入 runtime 失敗使用狀態 101：

| 條件 | 診斷 |
| --- | --- |
| stdin 錯誤 | `idwc: stdin I/O error` |
| 無效 UTF-8 | `idwc: invalid UTF-8 input` |
| 累積超過設定的 String 容量 | `idwc: input line buffer too long` |
| 超過設定的 Vec 容量 | `idwc: too many input tokens` |
| Token index 越界 | `idwc: token index out of bounds` |
| 無效／超出範圍整數 | `idwc: invalid integer` / `idwc: integer out of range` |
| 無效浮點 | `idwc: invalid float` |

這些 `.unwrap()` 使用受控失敗，不重現 Rust panic 文字或 unwinding。拒絕一般
allocation、token collection 賦值、String clone／串接、collection 函式
參數／回傳、任意 method、slice 與 iterator。

`TranspileOptions::with_string_capacity`／`--string-capacity` 和
`TranspileOptions::with_vec_capacity`／`--vec-capacity` 接受 1 到 65536；預設
分別為 4096 bytes 與 2048 elements。這些是生成 C 的轉譯設定；原生
`idwc::io::read_line()` 固定採預設 4096-byte 契約。容量越大、同時存活的
collection 越多，生成程式的 stack 用量也越高。

## 整數語意

- `i32` 對應 `int32_t`。加、減、乘、負號先在 `int64_t` 計算，檢查 i32
  範圍後才縮窄。
- 除法與餘數拒絕零，以及 `INT32_MIN / -1` overflow。
- 算術失敗會 flush stdout、寫出診斷，並以狀態 101 結束。
- 所有 C optimization level 都維持檢查，對應使用
  `-C overflow-checks=yes` 的 Rust，而不是 release wrapping。
- 限定 `usize` 對應 `size_t`；算術檢查 `SIZE_MAX`，除法／餘數拒絕零。
- 使用 `usize` 的 C 包含 static assertion，要求 pointer width 與執行 IdwC
  的 Rust host 相同；不相符的 cross-target 編譯會明確失敗。

## 浮點語意

- `f64` 以 binary64 C `double` 為目標，要求 `FLT_EVAL_METHOD == 0`、
  nearest-even rounding、subnormal 支援，並拒絕 fast-math／finite-math-only。
- 浮點字面量使用精確 C hex constant；volatile temporary 使每步運算捨入並
  避免跨 operation contraction。
- NaN、infinity、signed zero、除零、overflow 與 gradual underflow 採文件
  定義的 IEEE-style 行為；不保證 NaN payload／sign bit。
- 預設 `{}` 輸出能 roundtrip 至相同 binary64 的最短一般十進位；固定精度
  使用 round-half-to-even。特殊值輸出 `NaN`、`inf`、`-inf`，signed zero
  保留負號。
- 解析與最短輸出驗證要求 target `strtod` 正確捨入；行為會在 macOS libc
  與 CI targets 測試。

## 拒絕的語言功能

嚴格子集拒絕 struct、enum、union、`char`、未列出的 String／Vec 操作、
其他整數型別、`f32`、cast、浮點餘數、`f64::NAN` 等
associated constant、多數數學 method、產生值的 block、label、closure、任意
iterator chain、任意 macro、import、module、static、完整 `std`、async、
unsafe、raw pointer、泛型、trait、複雜 ownership／borrowing，以及輸入程式的
Cargo dependencies。

允許一般 comment；doc comment 屬於 attribute，因此拒絕。每個 AST node 都以
白名單處理，未支援語法不得默默消失。

## 轉譯診斷

- Library 錯誤使用三種穩定的 `TranspileErrorKind`：`Parse`、`Unsupported`
  或 `Semantic`。
- `TranspileError::message()` 回傳不含分類與位置前綴的訊息；
  `TranspileError::location()` 回傳可選的一基準行號與欄號。
- AST 驗證及語意錯誤會在能從 `syn` span 取得時回報相關語法位置。若是缺少
  `main` 等沒有對應 AST node 的整份程式錯誤，位置可以省略。
- `Display` 包含分類、可選位置與訊息。解析錯誤會透過
  `std::error::Error::source()` 保留底層 `syn::Error`。

以上是轉譯階段的診斷；輸入、陣列及算術章節中的 runtime 診斷文字與狀態
101 是另一組契約。

## 相容性契約

- IdwC 1.x 的公開 Rust API，以及本文記錄的嚴格模式接受行為，遵守
  Semantic Versioning。
- Minor release 可以接受更多來源程式，但不會默默重新解釋本文已接受的程式。
- Bug fix 可以在恢復文件所述行為時改變生成的 C。
- 生成結果的空白、comment、helper 順序與內部識別字屬於實作細節；使用者
  應編譯 C 輸出，不應依賴逐 byte 不變的原始碼。
- 本文明確列出的 runtime 診斷文字屬於嚴格契約；其他說明文字可在不提升
  major version 的情況下改善。

## 生成 C 與驗證

流程如下：

```text
Rust source → syn AST → 驗證與語意分析 → typed IR → C17 source
```

生成結果是單一 C 檔案，只嵌入實際需要的 helper。函式 prototype 位於定義
以前，生成名稱避免 C keyword collision，temporary 保留求值順序。CLI 先完成
轉譯並在相同目錄寫入暫存檔，再以 atomic rename 取代指定輸出。

測試會在相同 stdin 下比較可信任 Rust 與 C 程式，包括 stdout bytes、承諾的
stderr 及退出狀態。測試範圍包含 strict C17、Clang／GCC、optimized build、
UndefinedBehaviorSanitizer、邊界值、失敗路徑，以及提交範例輸出一致性。
