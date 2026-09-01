import 수학 [짝수이다 절댓값 거듭제곱하다]
import 통계 [평균 최댓값]
(declare
  (target
    name 수들
  )
  (value
    (list
      lit int 3
      lit int 1
      lit int 2
    )
  )
)
(declare
  (target
    name 글
  )
  (value
    lit str 가나다
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 3 + 4 = 
        (call 더하다 tail=값 neg=0 asks=0
          (slot 에
            lit int 3
          )
          (slot 를
            lit int 4
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 10 - 4 = 
        (call 빼다 tail=값 neg=0 asks=0
          (slot 에서
            lit int 10
          )
          (slot 를
            lit int 4
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 3 * 4 = 
        (call 곱하다 tail=값 neg=0 asks=0
          (slot 에
            lit int 3
          )
          (slot 를
            lit int 4
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 10 / 4 = 
        (call 나누다 tail=값 neg=0 asks=0
          (slot 를
            lit int 10
          )
          (slot 로
            lit int 4
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 10 % 4 = 
        (call 나누다 tail=나머지 neg=0 asks=0
          (slot 를
            lit int 10
          )
          (slot 로
            lit int 4
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 3 > 2: 
        (call 크다 tail=값 neg=0 asks=1
          (slot 가
            lit int 3
          )
          (slot 보다
            lit int 2
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 3 < 2: 
        (call 작다 tail=값 neg=0 asks=1
          (slot 가
            lit int 3
          )
          (slot 보다
            lit int 2
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 3 == 3: 
        (call 같다 tail=값 neg=0 asks=1
          (slot 가
            lit int 3
          )
          (slot 와
            lit int 3
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 정수로: 
        (call 바꾸다 tail=값 neg=0 asks=0
          (slot 를
            lit str 12
          )
          (slot 로
            name 정수
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 문자열로: 
        (field 글자수
          (call 바꾸다 tail=값 neg=0 asks=0
            (slot 를
              lit int 12
            )
            (slot 로
              name 문자열
            )
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 자료형: 
        (field 자료형
          lit int 12
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 개수: 
        (field 개수
          name 수들
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 2번째: 
        (field 2번째
          name 수들
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 복사본: 
        (field 복사본
          name 수들
        )
        lit str \n
      )
    )
  )
)
(declare
  (target
    name 철수
  )
  (value
    (dict
      (entry 이름
        lit str 새롬
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 열쇠 읽기: 
        (field 이름
          name 철수
        )
        lit str \n
      )
    )
  )
)
(declare
  (target
    (field 나이
      name 철수
    )
  )
  (value
    lit int 3
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 열쇠 쓰기: 
        name 철수
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 자르기: 
        (call 자르다 tail=값 neg=0 asks=0
          (slot 를
            lit str a,b
          )
          (slot 로
            lit str ,
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 잇기: 
        (call 잇다 tail=값 neg=0 asks=0
          (slot 를
            (list
              lit str a
              lit str b
            )
          )
          (slot 로
            lit str -
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 담기: 
        (call 담다 tail=값 neg=0 asks=1
          (slot 가
            name 글
          )
          (slot 를
            lit str 나
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 목록 담기: 
        (call 담다 tail=값 neg=0 asks=1
          (slot 가
            name 수들
          )
          (slot 를
            lit int 2
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 시작: 
        (call 시작하다 tail=값 neg=0 asks=1
          (slot 가
            name 글
          )
          (slot 로
            lit str 가
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 끝: 
        (call 끝나다 tail=값 neg=0 asks=1
          (slot 가
            name 글
          )
          (slot 로
            lit str 다
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 다듬기: [
        (call 다듬다 tail=값 neg=0 asks=0
          (slot 를
            lit str   가  
          )
        )
        lit str ]\n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 짝수: 
        (call 이다 tail=값 neg=0 asks=1
          (slot 가
            lit int 2
          )
          (slot -
            name 짝수
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 절댓값: 
        (field 절댓값
          lit int -7
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 거듭제곱: 
        (call 거듭제곱하다 tail=값 neg=0 asks=0
          (slot 를
            lit int 3
          )
          (slot 만큼
            lit int 2
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 평균: 
        (field 평균
          name 수들
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 최댓값: 
        (field 최댓값
          name 수들
        )
        lit str \n
      )
    )
  )
)
(declare
  (target
    name 빈것
  )
  (value
    (list
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 비었는가: 
        (call 이다 tail=값 neg=0 asks=1
          (slot 가
            (field 개수
              name 빈것
            )
          )
          (slot -
            lit int 0
          )
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 나누어떨어지는가: 
        (call 이다 tail=값 neg=0 asks=1
          (slot 가
            (call 나누다 tail=나머지 neg=0 asks=0
              (slot 를
                lit int 6
              )
              (slot 로
                lit int 3
              )
            )
          )
          (slot -
            lit int 0
          )
        )
        lit str \n
      )
    )
  )
)
(with 적을것
  (call 열다 tail=- neg=0 asks=0
    (slot 를
      lit str /tmp/새롬내장.txt
    )
  )
  (body
    (exec
      (call 쓰다 tail=- neg=0 asks=0
        (slot 를
          lit str 저장\n
        )
        (slot 에
          name 적을것
        )
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 읽음: 
        (call 읽다 tail=값 neg=0 asks=0
          (slot 를
            lit str /tmp/새롬내장.txt
          )
        )
      )
    )
  )
)
