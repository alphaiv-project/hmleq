# hmleq

한컴오피스 한글(HWP/HWPX) **수식 스크립트**(`x = {-b +- sqrt{b^2 - 4ac}} over {2a}` 형태의
수식 편집기 입력 언어)를 파싱해 **serde 직렬화 가능한 AST**로 만드는 Rust 크레이트입니다.
LaTeX 변환기는 기본 피처로 함께 제공됩니다.

- 언어 레퍼런스: [`docs/REFERENCE.md`](docs/REFERENCE.md) — 토큰 규칙, 키워드 매칭
  방식(구분자 기반 토큰화 + 낱말 전체 일치), 전체 명령어 목록
- 구현 설계/출력 규약: [`docs/DESIGN.md`](docs/DESIGN.md)

## 피처

| 피처 | 기본 | 내용 |
|---|---|---|
| `latex` | ✅ | `latex` 모듈, `to_latex` / `eq_to_latex`, CLI |
| `serde` | — | AST 전체 `Serialize`/`Deserialize` (라운드트립 보장) |
| `json` | — | CLI `--json` 출력 (`serde` 포함) |

파서/AST만 필요하면 `default-features = false`로 의존성 0개의 최소 빌드가 됩니다:

```toml
hmleq = { version = "0.2", default-features = false, features = ["serde"] }
```

## 사용법

라이브러리 — AST가 일급 산출물입니다:

```rust
let ast = hmleq::parse("1 over pi")?;

// feature "serde": AST ↔ JSON 라운드트립. 심볼은 정준 키워드 이름으로 직렬화되고
// 역직렬화 때 키워드 테이블로 되해석됩니다.
let json = serde_json::to_string(&ast)?;
assert_eq!(json, r#"{"Frac":{"num":{"Number":"1"},"den":{"Symbol":"pi"},"bar":true}}"#);
assert_eq!(serde_json::from_str::<hmleq::Node>(&json)?, ast);

// feature "latex" (기본): LaTeX 변환.
let latex = hmleq::eq_to_latex("sum _{n=1} ^{inf} {1 over n^2} = {pi^2} over 6")?;
assert_eq!(latex, r"\sum_{n = 1}^{\infty} \frac{1}{n^{2}} = \frac{\pi^{2}}{6}");
```

CLI:

```console
$ cargo run -q -- 'lim _{x rarrow 0} {sin x} over x = 1'
\lim_{x \rightarrow 0} \frac{\sin x}{x} = 1

$ cargo run -q --features json -- --json '1 over pi'
{"Frac":{"num":{"Number":"1"},"den":{"Symbol":"pi"},"bar":true}}

$ echo 'pmatrix { a & b # c & d }' | cargo run -q
\begin{pmatrix} a & b \\ c & d \end{pmatrix}
```

## 언어의 핵심 규칙 (요약)

- 빈칸·`{}`·`^`/`_` 등 **구분자로 낱말을 자른 뒤, 낱말 전체를 키워드 표에서 정확 일치로
  조회**한다. `sinh`는 sinh 하나이고, `sinx`는 키워드가 아니므로 이탤릭 *sinx*다.
- `over`/`atop`은 바로 앞·뒤 항에 붙는다: `{분자} over {분모}`.
- `#`은 줄바꿈, `&`는 정렬/행렬 열 구분, `~`/`` ` ``는 출력용 공백.
- `lim`/`Lim`, `rarrow`/`RARROW`처럼 대소문자가 의미를 바꾸는 키워드만 대소문자를 구분하고,
  나머지는 구분하지 않는다.

자세한 것은 `docs/REFERENCE.md` 참조.

## 테스트

```console
$ cargo test                  # 기본 (latex)
$ cargo test --all-features   # + serde 라운드트립
```

`tests/examples.rs`가 `docs/DESIGN.md` §6의 정준(canonical) 변환 표 전체를,
`tests/serde_roundtrip.rs`가 AST의 JSON 라운드트립을 검증합니다.
