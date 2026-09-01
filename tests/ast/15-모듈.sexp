import 수학 [*]
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 원주율: 
        (field 원주율
          name 수학
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
        lit str 7이 소수: 
        (call 이다 tail=값 neg=0 asks=1
          (slot 가
            lit int 7
          )
          (slot 모듈
            name 수학
          )
          (slot -
            name 소수
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
        lit str 2.5 반올림: 
        (call 반올림하다 tail=값 neg=0 asks=0
          (slot 모듈
            name 수학
          )
          (slot 를
            lit float 2.5
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
        lit str 2^10: 
        (call 거듭제곱하다 tail=값 neg=0 asks=0
          (slot 모듈
            name 수학
          )
          (slot 를
            lit int 2
          )
          (slot 만큼
            lit int 10
          )
        )
        lit str \n
      )
    )
  )
)
import 수학 [절댓값 제곱근 계승 약수들]
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str |-7|: 
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
        lit str √16: 
        (field 제곱근
          lit int 16
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
        lit str 5!: 
        (field 계승
          lit int 5
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
        lit str 12의 약수: 
        (field 약수들
          lit int 12
        )
        lit str \n
      )
    )
  )
)
import 통계 [합 평균 최댓값 최솟값 범위]
(declare
  (target
    name 점수들
  )
  (value
    (list
      lit int 80
      lit int 95
      lit int 72
      lit int 95
      lit int 61
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 합: 
        (field 합
          name 점수들
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
          name 점수들
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
          name 점수들
        )
        lit str   최솟값: 
        (field 최솟값
          name 점수들
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
        lit str 범위: 
        (field 범위
          name 점수들
        )
        lit str \n
      )
    )
  )
)
import 글자 [뒤집다 치환하다 왼쪽채움하다]
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 뒤집기: 
        (call 뒤집다 tail=값 neg=0 asks=0
          (slot 를
            lit str 새롬
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
        lit str 치환: 
        (call 치환하다 tail=값 neg=0 asks=0
          (slot 에서
            lit str a-b-c
          )
          (slot 를
            lit str -
          )
          (slot 로
            lit str /
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
      lit str 자리맞춤:\n
    )
  )
)
(loop range 자리
  (start
    lit int 1
  )
  (stop
    (field 개수
      name 점수들
    )
  )
  (body
    (declare
      (target
        name 점수
      )
      (value
        (field 자리번째
          name 점수들
        )
      )
    )
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            lit str   [
            (call 왼쪽채움하다 tail=값 neg=0 asks=0
              (slot 를
                (call 바꾸다 tail=값 neg=0 asks=0
                  (slot 를
                    name 점수
                  )
                  (slot 로
                    name 문자열
                  )
                )
              )
              (slot 만큼
                lit int 5
              )
            )
            lit str ]\n
          )
        )
      )
    )
  )
)
import 수학 [음수이다]
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str -5가 음수: 
        (call 이다 tail=값 neg=0 asks=1
          (slot 가
            lit int -5
          )
          (slot -
            name 음수
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
        lit str 3이 음수: 
        (call 이다 tail=값 neg=0 asks=1
          (slot 가
            lit int 3
          )
          (slot -
            name 음수
          )
        )
        lit str \n
      )
    )
  )
)
