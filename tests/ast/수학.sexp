(declare
  (target
    name 원주율
  )
  (value
    lit float 3.141592653589793
  )
)
(define 음수이다 predicate [가:수]
  (body
    (return
      (call 작다 tail=값 neg=0 asks=1
        (slot 가
          name 수
        )
        (slot 보다
          lit int 0
        )
      )
    )
  )
)
(define 짝수이다 predicate [가:수]
  (body
    (return
      (call 이다 tail=값 neg=0 asks=1
        (slot 가
          (call 나누다 tail=나머지 neg=0 asks=0
            (slot 를
              name 수
            )
            (slot 로
              lit int 2
            )
          )
        )
        (slot -
          lit int 0
        )
      )
    )
  )
)
(define 홀수이다 predicate [가:수]
  (body
    (return
      (call 이다 tail=값 neg=0 asks=1
        (slot 가
          (call 나누다 tail=나머지 neg=0 asks=0
            (slot 를
              name 수
            )
            (slot 로
              lit int 2
            )
          )
        )
        (slot -
          lit int 1
        )
      )
    )
  )
)
(define 배수이다 predicate [가:수 의:나눌수]
  (body
    (return
      (call 이다 tail=값 neg=0 asks=1
        (slot 가
          (call 나누다 tail=나머지 neg=0 asks=0
            (slot 를
              name 수
            )
            (slot 로
              name 나눌수
            )
          )
        )
        (slot -
          lit int 0
        )
      )
    )
  )
)
(noun 부호 of 수
  (body
    (if
      (branch
        (test
          (call 크다 tail=- neg=0 asks=0
            (slot 가
              name 수
            )
            (slot 보다
              lit int 0
            )
          )
        )
        (body
          (return
            lit int 1
          )
        )
      )
    )
    (if
      (branch
        (test
          (call 이다 tail=- neg=0 asks=0
            (slot 가
              name 수
            )
            (slot -
              name 음수
            )
          )
        )
        (body
          (return
            lit int -1
          )
        )
      )
    )
    (return
      lit int 0
    )
  )
)
(noun 절댓값 of 수
  (body
    (if
      (branch
        (test
          (call 이다 tail=- neg=0 asks=0
            (slot 가
              name 수
            )
            (slot -
              name 음수
            )
          )
        )
        (body
          (return
            (call 빼다 tail=값 neg=0 asks=0
              (slot 에서
                lit int 0
              )
              (slot 를
                name 수
              )
            )
          )
        )
      )
    )
    (return
      name 수
    )
  )
)
(define 내림하다 verb [를:수]
  (body
    (declare
      (target
        name 자른값
      )
      (value
        (call 바꾸다 tail=값 neg=0 asks=0
          (slot 를
            name 수
          )
          (slot 로
            name 정수
          )
        )
      )
    )
    (if
      (branch
        (test
          (call 그리고 tail=- neg=0 asks=0
            (slot -
              (call 이다 tail=- neg=0 asks=0
                (slot 가
                  name 수
                )
                (slot -
                  name 음수
                )
              )
            )
            (slot -
              (call 같다 tail=- neg=1 asks=0
                (slot 가
                  name 자른값
                )
                (slot 와
                  name 수
                )
              )
            )
          )
        )
        (body
          (return
            (call 빼다 tail=값 neg=0 asks=0
              (slot 에서
                name 자른값
              )
              (slot 를
                lit int 1
              )
            )
          )
        )
      )
    )
    (return
      name 자른값
    )
  )
)
(define 올림하다 verb [를:수]
  (body
    (declare
      (target
        name 자른값
      )
      (value
        (call 바꾸다 tail=값 neg=0 asks=0
          (slot 를
            name 수
          )
          (slot 로
            name 정수
          )
        )
      )
    )
    (if
      (branch
        (test
          (call 그리고 tail=- neg=0 asks=0
            (slot -
              (call 크다 tail=- neg=0 asks=0
                (slot 가
                  name 수
                )
                (slot 보다
                  lit int 0
                )
              )
            )
            (slot -
              (call 같다 tail=- neg=1 asks=0
                (slot 가
                  name 자른값
                )
                (slot 와
                  name 수
                )
              )
            )
          )
        )
        (body
          (return
            (call 더하다 tail=값 neg=0 asks=0
              (slot 에
                name 자른값
              )
              (slot 를
                lit int 1
              )
            )
          )
        )
      )
    )
    (return
      name 자른값
    )
  )
)
(define 반올림하다 verb [를:수]
  (body
    (return
      (call 내림하다 tail=값 neg=0 asks=0
        (slot 를
          (call 더하다 tail=값 neg=0 asks=0
            (slot 에
              name 수
            )
            (slot 를
              lit float 0.5
            )
          )
        )
      )
    )
  )
)
(define 거듭제곱하다 verb [를:밑 만큼:지수]
  (body
    (if
      (branch
        (test
          (call 작다 tail=- neg=0 asks=0
            (slot 가
              name 지수
            )
            (slot 보다
              lit int 1
            )
          )
        )
        (body
          (return
            lit int 1
          )
        )
      )
    )
    (declare
      (target
        name 쌓은값
      )
      (value
        lit int 1
      )
    )
    (loop range 번
      (start
        lit int 1
      )
      (stop
        name 지수
      )
      (body
        (declare
          (target
            name 쌓은값
          )
          (value
            (call 곱하다 tail=값 neg=0 asks=0
              (slot 에
                name 쌓은값
              )
              (slot 를
                name 밑
              )
            )
          )
        )
      )
    )
    (return
      name 쌓은값
    )
  )
)
(noun 제곱근 of 수
  (body
    (if
      (branch
        (test
          (call 이다 tail=- neg=0 asks=0
            (slot 가
              name 수
            )
            (slot -
              name 음수
            )
          )
        )
        (body
          (exec
            (call 종료하다 tail=- neg=0 asks=0
              (slot 로
                lit str 제곱근은 음수에 쓸 수 없음
              )
            )
          )
        )
      )
    )
    (if
      (branch
        (test
          (call 이다 tail=- neg=0 asks=0
            (slot 가
              name 수
            )
            (slot -
              lit int 0
            )
          )
        )
        (body
          (return
            lit int 0
          )
        )
      )
    )
    (declare
      (target
        name 어림
      )
      (value
        name 수
      )
    )
    (loop range 번
      (start
        lit int 1
      )
      (stop
        lit int 40
      )
      (body
        (declare
          (target
            name 어림
          )
          (value
            (call 나누다 tail=값 neg=0 asks=0
              (slot 를
                (call 더하다 tail=값 neg=0 asks=0
                  (slot 에
                    name 어림
                  )
                  (slot 를
                    (call 나누다 tail=값 neg=0 asks=0
                      (slot 를
                        name 수
                      )
                      (slot 로
                        name 어림
                      )
                    )
                  )
                )
              )
              (slot 로
                lit int 2
              )
            )
          )
        )
      )
    )
    (return
      name 어림
    )
  )
)
(noun 계승 of 수
  (body
    (if
      (branch
        (test
          (call 작다 tail=- neg=0 asks=0
            (slot 가
              name 수
            )
            (slot 보다
              lit int 2
            )
          )
        )
        (body
          (return
            lit int 1
          )
        )
      )
    )
    (declare
      (target
        name 쌓은값
      )
      (value
        lit int 1
      )
    )
    (loop range 번
      (start
        lit int 1
      )
      (stop
        name 수
      )
      (body
        (declare
          (target
            name 쌓은값
          )
          (value
            (call 곱하다 tail=값 neg=0 asks=0
              (slot 에
                name 쌓은값
              )
              (slot 를
                name 번
              )
            )
          )
        )
      )
    )
    (return
      name 쌓은값
    )
  )
)
(define 소수이다 predicate [가:수]
  (body
    (if
      (branch
        (test
          (call 작다 tail=- neg=0 asks=0
            (slot 가
              name 수
            )
            (slot 보다
              lit int 2
            )
          )
        )
        (body
          (return
            lit bool 거짓
          )
        )
      )
    )
    (loop range 나눌수
      (start
        lit int 2
      )
      (stop
        name 수
      )
      (body
        (if
          (branch
            (test
              (call 크다 tail=- neg=0 asks=0
                (slot 가
                  (call 곱하다 tail=값 neg=0 asks=0
                    (slot 에
                      name 나눌수
                    )
                    (slot 를
                      name 나눌수
                    )
                  )
                )
                (slot 보다
                  name 수
                )
              )
            )
            (body
              break
            )
          )
        )
        (if
          (branch
            (test
              (call 이다 tail=- neg=0 asks=0
                (slot 가
                  name 수
                )
                (slot 의
                  name 나눌수
                )
                (slot -
                  name 배수
                )
              )
            )
            (body
              (return
                lit bool 거짓
              )
            )
          )
        )
      )
    )
    (return
      lit bool 참
    )
  )
)
(noun 약수들 of 수
  (body
    (declare
      (target
        name 모은것
      )
      (value
        (list
        )
      )
    )
    (loop range 나눌수
      (start
        lit int 1
      )
      (stop
        name 수
      )
      (body
        (if
          (branch
            (test
              (call 이다 tail=- neg=0 asks=0
                (slot 가
                  name 수
                )
                (slot 의
                  name 나눌수
                )
                (slot -
                  name 배수
                )
              )
            )
            (body
              (exec
                (call 더하다 tail=- neg=0 asks=0
                  (slot 에
                    name 모은것
                  )
                  (slot 를
                    name 나눌수
                  )
                )
              )
            )
          )
        )
      )
    )
    (return
      name 모은것
    )
  )
)
(noun 최대공약수 of 수들
  (body
    (declare
      (target
        name 큰수
      )
      (value
        (field 절댓값
          (field 첫째
            name 수들
          )
        )
      )
    )
    (declare
      (target
        name 작은수
      )
      (value
        (field 절댓값
          (field 마지막
            name 수들
          )
        )
      )
    )
    (loop while
      (test
        (call 크다 tail=값 neg=0 asks=0
          (slot 가
            name 작은수
          )
          (slot 보다
            lit int 0
          )
        )
      )
      (body
        (declare
          (target
            name 나머지
          )
          (value
            (call 나누다 tail=나머지 neg=0 asks=0
              (slot 를
                name 큰수
              )
              (slot 로
                name 작은수
              )
            )
          )
        )
        (declare
          (target
            name 큰수
          )
          (value
            name 작은수
          )
        )
        (declare
          (target
            name 작은수
          )
          (value
            name 나머지
          )
        )
      )
    )
    (return
      name 큰수
    )
  )
)
(noun 최소공배수 of 수들
  (body
    (declare
      (target
        name 큰수
      )
      (value
        (field 첫째
          name 수들
        )
      )
    )
    (declare
      (target
        name 작은수
      )
      (value
        (field 마지막
          name 수들
        )
      )
    )
    (declare
      (target
        name 나눌값
      )
      (value
        (field 최대공약수
          name 수들
        )
      )
    )
    (return
      (call 곱하다 tail=값 neg=0 asks=0
        (slot 에
          (call 나누다 tail=값 neg=0 asks=0
            (slot 를
              name 큰수
            )
            (slot 로
              name 나눌값
            )
          )
        )
        (slot 를
          name 작은수
        )
      )
    )
  )
)
