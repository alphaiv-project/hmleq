# 한컴오피스 한글 수식 스크립트 — 공식 레퍼런스 위키

> 한컴오피스 한글(HWP/HWPX) 수식 편집기의 **스크립트 입력 언어**를 정리한 문서입니다.
> 근거 자료: 한컴 공식 도움말(수식 명령어 목록/설명, 수식 글꼴과 기본 함수), 한컴 공개 문서
> 「한글 문서 파일 형식 — 수식 (revision 1.3)」, 및 커뮤니티 실사용 팁.

---

## 목차

1. [개요](#1-개요)
2. [토큰(항) 구분 규칙 — 문법의 핵심](#2-토큰항-구분-규칙--문법의-핵심)
3. [키워드 매칭 방식: non-prefix인가, 최장 접두사 매칭인가?](#3-키워드-매칭-방식-non-prefix인가-최장-접두사-매칭인가)
4. [명령어 레퍼런스](#4-명령어-레퍼런스)
5. [수식 예시 모음](#5-수식-예시-모음)
6. [함정과 팁](#6-함정과-팁)
7. [참고 자료](#7-참고-자료)

---

## 1. 개요

한글 수식 편집기(단축키 `Ctrl+N,M`)는 WYSIWYG 편집 창과 **스크립트 입력 창**을 함께 제공한다.
스크립트는 LaTeX가 아니라 옛 troff/eqn 계열에 가까운 독자 문법으로, 다음과 같은 형태다.

```text
x = {-b +- sqrt{b^2 - 4ac}} over {2a}
```

- 명령어(키워드)는 **대부분 대소문자를 구분하지 않는다.** 단, 도움말에 첫 글자가 대문자로
  표기된 것(`Lim` 등)은 반드시 그 표기대로 입력해야 한다.
- 영문자는 기본적으로 **이탤릭체**로 표시되고, `sin`, `log` 같은 **기본 함수·예약어는
  자동으로 로만체**로 표시된다.

---

## 2. 토큰(항) 구분 규칙 — 문법의 핵심

수식 스크립트는 먼저 **항(token) 단위로 잘라 읽는다.** 항을 구분하는 요소는 다음과 같다.

| 구분자 | 역할 |
|---|---|
| 빈칸, 줄바꿈(Enter), 탭 | 항과 항을 구분한다 (출력에는 나타나지 않음) |
| `{ }` | 여러 항을 하나의 항으로 묶는다 (분자·분모, 첨자 범위 등에서 필수) |
| `^` / `_` | 위첨자 / 아래첨자 (바로 뒤 한 항에만 적용, 여러 항이면 `{}`로 묶기) |
| `#` | 줄 바꾸기 |
| `&` | 여러 줄에서 세로 위치 맞춤 (행렬의 열 구분에도 사용) |
| `~` | 출력되는 빈칸 (한 칸) |
| `` ` `` | 출력되는 1/4 크기 빈칸 |
| `" "` | 문자열을 통째로 하나의 낱말로 처리. **9자 이상 낱말은 반드시 따옴표로 묶어야 한다** |

핵심 원칙 한 줄 요약: **"앞 식과 끊을 땐 빈칸, 뒤 식과 묶을 땐 `{}`."**

---

## 3. 키워드 매칭 방식: non-prefix인가, 최장 접두사 매칭인가?

결론부터: **둘 다 아니다. "구분자 기반 토큰화 + 토큰 전체 일치(whole-token exact match)"
방식이다.**

### 3.1 키워드 집합은 prefix-free가 아니다

키워드끼리 접두사 관계인 쌍이 실제로 많다.

| 접두사 쌍 | 의미 |
|---|---|
| `in` / `inf` / `int` / `inter` | ∈ / ∞ / ∫ / ∩ |
| `sin` / `sinh`, `cos` / `cosh`, `tan` / `tanh`, `cot` / `coth` | 삼각함수 / 쌍곡선함수 |
| `pi` / `pile` | π / 세로 쌓기 명령 |
| `dot` / `doteq` | 점 장식 / ≐ |
| `sim` / `simeq` | ∼ / ≃ |
| `sub` / `subset` / `subseteq` | 아래첨자 / ⊂ / ⊆ |
| `sup` / `supset` / `supseteq` | 위첨자 / ⊃ / ⊇ |

즉 "키워드를 서로 non-prefix가 되도록 설계했다"는 가설은 성립하지 않는다.

### 3.2 그렇다고 문자 단위 최장 접두사 스캔도 아니다

파서는 문자열을 한 글자씩 훑으며 "가장 긴 키워드 접두사"를 뜯어내는 방식(maximal munch)이
아니라,

1. 먼저 **구분자(빈칸, `{}`, `^`, `_`, 숫자·연산자 경계 등)로 연속된 영문자 덩어리(낱말)를
   자른 뒤**,
2. 그 **낱말 전체를 키워드 표에서 조회**한다.
3. 전체가 일치하면 키워드(기호/명령/함수)로, 일치하지 않으면 그냥 이탤릭 문자열로 처리한다.

그래서:

- `sinh` → 낱말 전체 "sinh"가 표에 있으므로 **sinh 하나로 인식**된다. `sin` + `h`로 쪼개지지
  않는다. (접두사 충돌이 토큰 단위에서 자연히 해소됨 — 결과만 보면 "항상 가장 긴 해석이
  이긴다"처럼 보이는 이유)
- `pile` → **pile 명령**이지 `pi` + `le`가 아니다. π 뒤에 le를 쓰고 싶으면 `pi le` 또는
  `pi{le}`처럼 끊어야 한다.
- 반대로 `sinx` → "sinx"라는 낱말은 표에 없으므로 **sin 함수로 인식되지 않고** 이탤릭
  *sinx*가 된다. sin x를 원하면 `sin x`로 띄어야 한다.
- 같은 원리로, 함수를 일부러 이탤릭으로 쓰고 싶으면 `s in`, `si n`처럼 낱말을 깨면 된다고
  공식 파일 형식 문서가 안내한다. (단, `in`은 ∈ 기호 키워드이기도 하므로 실제로는 `si n`
  쪽이 안전하다.)

### 3.3 요약

| 가설 | 판정 |
|---|---|
| 키워드들이 서로 non-prefix(접두사 없는 집합)이다 | ❌ — `in`/`int`, `pi`/`pile` 등 접두사 쌍이 다수 존재 |
| 문자 단위 최장 접두사(greedy) 매칭이다 | ❌ — `sinx`가 `sin`+`x`로 분해되지 않음 |
| **구분자로 낱말을 자른 뒤 낱말 전체를 정확 일치로 조회** | ✅ — 접두사 모호성은 토큰 경계에서 해소 |

> 참고: 공식 파일 형식 문서(rev 1.3)도 형식적 어휘분석 규칙(정규문법)을 명시하지는 않는다.
> 위 동작은 문서의 토큰 구분 규칙 + 편집기의 실제 동작(커뮤니티에서 널리 검증된 사례)에
> 근거한 정리다.

---

## 4. 명령어 레퍼런스

### 4.1 기본 구조

| 스크립트 | 결과 | 예시 |
|---|---|---|
| `A over B` | 분수 A/B | `1 over 2` |
| `A atop B` | 분수선 없는 위아래 배치 | `x atop y` |
| `sqrt {A}` | 제곱근 √A | `sqrt 2` |
| `root n of {A}` | n제곱근 | `root 3 of {x+1}` |
| `A ^ {B}` (또는 `sup`) | 위첨자 | `E = mc^2` |
| `A _ {B}` (또는 `sub`) | 아래첨자 | `H_2 O` |
| `LEFT ( … RIGHT )` | 내용 크기에 맞게 늘어나는 괄호 | `LEFT( x over y RIGHT)` |
| `bigg` | 기호 크기 확대 | `{a+b} over {a-b} bigg / {x+y}` |
| `not` | 뒤 기호에 부정 사선 | `not =` → ≠ |

### 4.2 대형 연산자 (극한·합·적분·집합)

| 스크립트 | 결과 |
|---|---|
| `lim` / `Lim` | 극한 (아래 첨자 위치가 다름 — 대소문자 구분 필수) |
| `sum` | ∑ — `sum_{k=1}^{n}` |
| `prod` | ∏ |
| `int`, `oint` | ∫, ∮ |
| `dint`, `tint`, `odint`, `otint` | ∬, ∭, ∯, ∰ |
| `union`, `inter` | ⋃, ⋂ (대형) |
| `small` + 연산자 (예: `smallsum`, `smallint`) | 첨자가 옆에 붙는 작은 기호 |

### 4.3 행렬·배치·조합

| 스크립트 | 결과 |
|---|---|
| `matrix { a & b # c & d }` | 괄호 없는 행렬 (`&` 열 구분, `#` 행 구분) |
| `pmatrix { … }` | ( ) 행렬 |
| `bmatrix { … }` | [ ] 행렬 |
| `dmatrix { … }` | \| \| 행렬(행렬식) |
| `cases { … # … }` | 경우 나누기 (왼쪽 `{`) |
| `pile { … # … }` / `lpile` / `rpile` | 세로 쌓기 (가운데/왼쪽/오른쪽 정렬) |
| `eqalign { … & … # … }` | `&` 기준 여러 줄 세로 맞춤 |
| `n choose k`, `binom {n}{k}` | 조합 기호 |
| `rel`, `buildrel` | 화살표 위·아래에 관계식 얹기 |

### 4.4 글자 장식

명령어를 **먼저** 쓰고 문자를 뒤에 쓴다: `vec A`, `bar x`, `hat y`.

`acute`, `grave`, `dot`, `ddot`, `bar`, `vec`, `dyad`, `hat`, `check`, `arch`, `tilde`, `under`

### 4.5 그리스 문자

| 종류 | 스크립트 |
|---|---|
| 소문자 | `alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega` |
| 대문자 | `Alpha Beta Gamma Delta … Omega` (첫 글자만 대문자) |
| 변형 | `vartheta varpi varsigma varupsilon varphi varepsilon` |
| 특수 | `aleph hbar imath jmath ohm ell liter wp imag angstrom` |

### 4.6 화살표

`larrow`(←), `rarrow`(→), `uparrow`, `downarrow`, `lrarrow`(↔), `udarrow`,
대문자형 `LARROW`(⇐), `RARROW`(⇒), `LRARROW`(⇔), `UDARROW`,
대각선 `nwarrow nearrow swarrow searrow`, 기타 `hookleft hookright mapsto`
(`leftarrow`, `rightarrow`, `Rightarrow` 같은 긴 이름도 인식된다.)

### 4.7 관계·비교 기호

`leq`(≤), `geq`(≥), `neq` 또는 `!=`(≠), `doteq`(≐), `sim`(∼), `simeq`(≃), `approx`(≈),
`cong`(≅), `equiv`(≡), `asymp`(≍), `identical`

### 4.8 집합 기호

`in`(∈), `owns`(∋), `notin`(∉), `subset`(⊂), `supset`(⊃), `subseteq`(⊆), `supseteq`(⊇),
`union`(∪), `inter`(∩), `emptyset`(∅)

### 4.9 기타 특수 기호

`inf`(∞), `partial`(∂), `therefore`(∴), `because`(∵), `pm`/`plusminus`(±), `mp`/`minusplus`(∓),
`times`(×), `div`/`divide`(÷), `cdot`(·), `forall`(∀), `exist`(∃), `prime`(′), `deg`,
`diamond`, `dsum`

### 4.10 글꼴 명령과 기본 함수

| 명령 | 효과 |
|---|---|
| `rm` | 이후 입력을 로만체로 |
| `it` | 다시 이탤릭체로 |
| `bold` | 볼드체 |
| `rmbold` | 로만체 볼드 |

**자동으로 로만체가 되는 기본 함수·예약어:**

`sin cos tan cot sec csc cosec sinh cosh tanh coth arcsin arccos arctan log ln lg exp Exp
lim Lim max min det gcd mod deg arg dim hom ker Pr if for and or`

---

## 5. 수식 예시 모음

**근의 공식**

```text
x = {-b +- sqrt{b^2 -4ac}} over {2a}
```

**극한**

```text
lim _{x rarrow 0} {sin x} over x = 1
```

**정적분**

```text
int _1 ^2 {3x^2} dx = LEFT[ x^3 RIGHT] _1 ^2 = 7
```

**급수**

```text
sum _{n=1} ^{inf} {1 over n^2} = {pi^2} over 6
```

**행렬**

```text
pmatrix { a_1 & b_1 # a_2 & b_2 }
```

**경우 나누기**

```text
f(x) = cases { x^2 & (x geq 0) # -x & (x < 0) }
```

**집합**

```text
A inter B = { x | x in A ~and~ x in B }
```

---

## 6. 함정과 팁

- **`sinx`는 sin x가 아니다.** 낱말 전체가 키워드와 일치해야 하므로 반드시 `sin x`로 띄어
  쓴다. (§3 참조)
- **`pile`은 π+le가 아니라 쌓기 명령이다.** π 뒤에 문자를 붙이려면 `pi le`처럼 끊는다.
- **9자 이상 낱말은 `"…"`로 묶어야** 하나의 낱말로 처리된다.
- `lim`/`Lim`처럼 **대소문자가 의미를 바꾸는 예외**가 있다 (도움말에 첫 글자 대문자로 표기된
  명령은 그대로 입력).
- 첨자 `^`, `_`는 **바로 뒤 한 항**에만 걸린다. `a^2 2`는 2만 위첨자가 되고 그다음 2는 본문
  크기. 여러 항은 `{}`로 묶는다: `a^{2b}`.
- 빈칸은 출력되지 않는다. 출력용 공백은 `~`(한 칸) 또는 `` ` ``(1/4칸)을 쓴다.
- 스크립트 입력 창이 아닌 **[명령어 입력] 상자에 쓰면 기본 함수로 인식되지 않는** 경우가
  있으니 수식 창에서 직접 입력한다.
- 함수 이름을 일부러 이탤릭으로 쓰려면 낱말을 깬다: `si n`. (`s in`은 `in`(∈)과 충돌할 수
  있어 비추천.)

---

## 7. 참고 자료

- 한컴 도움말 — [수식 명령어 목록](https://help.hancom.com/hoffice/multi/ko_kr/hwp/insert/equation/equation(script).htm)
- 한컴 도움말 — [수식 명령어 설명](https://help.hancom.com/hoffice/multi/ko_kr/hwp/insert/equation/equation(explanation).htm)
- 한컴 도움말 — [수식 글꼴과 기본 함수](https://help.hancom.com/hoffice/multi/ko_kr/hwp/insert/equation/equation(font).htm)
- 한컴 공개 문서 — [한글 문서 파일 형식: 수식 revision 1.3 (PDF)](https://cdn.hancom.com/link/docs/%ED%95%9C%EA%B8%80%EB%AC%B8%EC%84%9C%ED%8C%8C%EC%9D%BC%ED%98%95%EC%8B%9D_%EC%88%98%EC%8B%9D_revision1.3.pdf)
- SASA Math — [한컴오피스 한글로 수식이 삽입된 문서 작성하는 방법](https://sasamath.com/blog/tip-collection/how-to-write-equations-in-hwp/)
- 커뮤니티 팁 — [아래한글 수식입력 정리 (한스디 카페)](https://m.cafe.daum.net/hwp-script-db/bVJe/31)
- 참고 구현 — [hml-equation-parser (HWP 수식 → LaTeX 변환기)](https://github.com/OpenBapul/hml-equation-parser)
