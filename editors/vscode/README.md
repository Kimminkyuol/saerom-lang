# 새롬

문법 강조와 `saeromc --lsp` 언어 서버.

- 자동완성: 쓴 조사에 맞는 동사와 활용형, `의` 뒤 필드, 명칭, 모듈 멤버, `<모듈>에서` 뒤 정의
- 오류 표시, 설명(추론한 자료형 포함), 정의로 이동, 개요, 의미 강조
- 참조, 이름 바꾸기: 정의가 있는 글과 같은 폴더의 글까지. 동사는 활용형을 맞춰 바꿈
- 서식 맞추기: 들여쓰기, 줄 이음, 띄어쓰기
- `새롬: 실행` (⌘F5)

## 설치

```sh
make install                # 저장소 루트. ~/.local/bin/saeromc
cd editors/vscode && npm install
ln -s "$PWD" ~/.vscode/extensions/saerom
```

`saerom.compiler`로 `saeromc` 경로를 지정할 수 있다.
