# hmleq

한컴오피스 한글(HWP/HWPX) **수식 스크립트**(`x = {-b +- sqrt{b^2 - 4ac}} over {2a}` 형태의
수식 편집기 입력 언어)를 파싱해 AST로 만들고 **LaTeX로 변환**하는 Rust 크레이트입니다.

- 언어 레퍼런스: [`docs/REFERENCE.md`](docs/REFERENCE.md) — 토큰 규칙, 키워드 매칭
  방식(구분자 기반 토큰화 + 낱말 전체 일치), 전체 명령어 목록
- 구현 설계/출력 규약: [`docs/DESIGN.md`](docs/DESIGN.md)

## 사용법

라이브러리:

```rust
let latex = hmleq::eq_to_latex("sum _{n=1} ^{inf} {1 over n^2} = {pi^2} over 6")?;
assert_eq!(latex, r"\sum_{n = 1}^{\infty} \frac{1}{n^{2}} = \frac{\pi^{2}}{6}");
```

AST가 필요하면 `hmleq::parse` / `hmleq::to_latex`를 따로 호출합니다.

CLI:

```console
$ cargo run -q -- 'lim _{x rarrow 0} {sin x} over x = 1'
\lim_{x \rightarrow 0} \frac{\sin x}{x} = 1

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
$ cargo test
```

`tests/examples.rs`가 `docs/DESIGN.md` §6의 정준(canonical) 변환 표 전체를 검증합니다.
