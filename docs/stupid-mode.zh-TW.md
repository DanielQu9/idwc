# IdwC Stupid Mode 規格

[English](stupid-mode.md) · [返回 README](../README.zh-TW.md) ·
[嚴格語意規格](strict-semantics.zh-TW.md)

Stupid Mode 是 IdwC v1.1.0 新增的可選可讀 C 生成器。CLI 使用
`-s`／`--stupid` 啟用，library API 則指定 [`TranspileMode::Stupid`]；嚴格
模式仍是預設值。

```bash
idwc --stupid input.rs -o output.c
```

名字是玩笑，取捨是認真的。Stupid Mode 接受與嚴格模式相同、經驗證的 Rust
子集，但優先產生精簡、容易修改的 C，不保證完整保留 Rust runtime 行為。

## 生成原始碼

- 只要在 C 中合法且沒有歧義，就保留原始變數、參數、range binding 與函式
  名稱。
- C keyword、保留識別字、標準函式衝突與 Rust shadowing 會使用可預期的
  後綴改名，例如 `value_2`。
- 可見 Unicode 直接寫入 C17 `u8"..."` 字串；引號、反斜線、控制字元、
  問號及 `printf` 百分號仍在必要時跳脫。
- 只引入用到的 header。IdwC 不生成私有 runtime helper function 或 struct
  定義；Rust 原始碼本來定義的函式仍會成為一般 C 函式。
- `i32`、`usize`、`f64`、`bool` 分別映射為 `int`、`size_t`、`double`、
  `bool`；固定陣列使用區域 C array。支援的 `Vec<T>` 會成為一個區域 C array
  加上 `values_len` 之類的可讀 length 變數，不生成 Vec struct 或 helper。
- 限定 collection `{:?}` 直接成為寫出方括號與分隔符的一般 C `for` loop；
  共用的語意驗證仍會拒絕 `f64` collection Debug 格式。
- 可變 `&str` binding 仍使用 `const char *`：Rust binding 的 mutability 允許
  pointer 重新指向，不代表可以修改字串字面量 bytes。
- 簡單 Vec `push`、repeat value 與索引賦值直接生成 C 寫入；可重用的混合
  formatter 引數也保持 inline。若右側有副作用，會先求值至 temporary 再計算
  index；Vec binding 間的 move 使用一次明確 copy loop。

## 刻意省略的保證

Stupid Mode 直接生成 C 算術與索引，不檢查 signed overflow、負號 overflow、
除數或餘數為零、`INT_MIN / -1`、unsigned wraparound、陣列邊界及 inclusive
range overflow。選用此模式即接受這些情況可能造成 C undefined behavior。

Vec capacity、`push` 與索引也不檢查。超過設定的區域 array 或越過 logical
length 索引可能觸發 C undefined behavior。

函式引數及其他直接 C expression 可能採用 C 求值順序，而不是 Rust 的由左
至右順序。只有在合法 C 或避免明顯重複副作用所需時才保留 temporary。

浮點值使用一般 `double` 運算。IdwC 不設定浮點環境、不拒絕 fast-math、
不強制每步捨入，也不模擬 Rust `Display`。`{}` 使用 `%g`，固定精度使用 C
`%.Nf`；locale、捨入、特殊值、overflow、underflow 與額外精度依 C 實作及
compiler options 決定。

## 輸入行為

- `read_i32()` 和 `read_f64()` 直接成為未檢查的 `%d`／`%lf` `scanf`；轉換
  失敗可能留下未初始化值。
- `flush_stdout()` 直接呼叫 `fflush(stdout)`，不檢查結果。
- 限定 `String` 輸入使用區域 `char` array；設定的 payload 容量（預設 4096）
  之外多保留一個 NUL byte，並使用未檢查的 `fgets`。`idwc::io::read_line()`
  會直接降低為空 array 加上一次這類呼叫。
- Token collection 使用設定的 Vec 容量（預設 2048），且不檢查 overflow。
- `&str` binding 直接成為 `char` pointer；owned String 使用區域 array，
  `String::from`、賦值與 move 透過未檢查的 `strcpy`，`{}` 則使用 `%s`。
  容量 overflow 與內嵌 NUL 行為因此依 C，而非 Strict Mode。
- `split_whitespace()` 使用 `strtok`、ASCII whitespace，以及足以容納固定行
  buffer 內所有可能 token 的區域 pointer array；不保留 Unicode whitespace
  行為。
- `parse::<i32>()`／`parse::<f64>()` 直接呼叫未檢查的 `strtol`／`strtod`；
  token 索引也不檢查邊界。

Stupid Mode 適合輸入已知的小型課堂程式，不適合處理不可信任輸入，也不適合
依賴 Rust panic、overflow、浮點、邊界、UTF-8 或求值順序保證的程式。

## 仍保留的檢查

兩種模式都使用 `syn`、支援 AST 白名單、名稱與 scope 解析、可變性驗證及型別
檢查。不支援的語法、未定義名稱及錯誤型別組合會在生成 C 前被拒絕。Stupid
Mode 不會執行輸入 Rust，也不會展開使用者自訂 macro。

CLI 選用此模式時會在 stderr 顯示一次提示，生成檔案也會用簡短註解標明其
寬鬆契約。

[`TranspileMode::Stupid`]: https://docs.rs/idwc/latest/idwc/enum.TranspileMode.html#variant.Stupid
