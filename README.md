# 새롬

한국어 문법을 따르는 프로그래밍 언어.

## 실행

```
saeromc <파일.sr> [-o <출력>] [-O2]
saeromc --check <파일.sr>
saeromc --emit-llvm <파일.sr>
```

## 오류

실행 오류는 프로그램을 끝낸다. 잡는 문법은 없다.

```
SAEROM_BACKTRACE=1 ./프로그램    역추적까지 본다
```

## 빌드

```
make          컴파일러와 런타임을 짓는다
make test     골든 검사
```

## 라이선스

MIT
