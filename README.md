# saerom-lang

[새롬](../새롬)을 네이티브로 컴파일한다. LLVM 텍스트 IR을 찍고 `clang`으로 링크한다.

```
소스(.sr) → prescan → lex → parse → resolve → LLVM IR → clang → 실행파일
```

## 짓기

```
make          컴파일러와 런타임을 짓는다
make test     골든 검사
```

## 쓰기

```
saeromc <파일.sr> [-o <출력>]   실행파일로 컴파일
saeromc --emit-llvm <파일.sr>   LLVM IR만 찍는다
```

```
$ cat 안녕.sr
"안녕, 새롬\n"을 출력한다.
$ ./target/debug/saeromc 안녕.sr -o 안녕 && ./안녕
안녕, 새롬
```

## 자리

| | |
|---|---|
| `crates/saeromc` | 컴파일러 |
| `crates/saerom-rt` | 런타임 (정적 링크) |
| `tests/*.sr` | 골든 검사. `# →` 주석이 기대 출력이다 |
| [PLAN.md](PLAN.md) | 설계와 단계 |

## 어디까지

**M0** 걷는 뼈대 — 문자열 출력 한 문장이 네이티브 실행파일이 된다.

다음은 M1 어휘 분석 전체. [PLAN.md](PLAN.md)의 단계 표를 따른다.

## 라이선스

MIT
