import 수학 [짝수이다]
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
(if
  (branch
    (test
      (call 이다 tail=- neg=0 asks=0
        (slot 가
          lit int 7
        )
        (slot -
          name 홀수
        )
      )
    )
    (body
      (exec
        (call 출력하다 tail=- neg=0 asks=0
          (slot 를
            lit str 7은 홀수\n
          )
        )
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 3이 홀수인가: 
        (call 이다 tail=값 neg=0 asks=1
          (slot 가
            lit int 3
          )
          (slot -
            name 홀수
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
        lit str 4가 홀수인가: 
        (call 이다 tail=값 neg=0 asks=1
          (slot 가
            lit int 4
          )
          (slot -
            name 홀수
          )
        )
        lit str \n
      )
    )
  )
)
(if
  (branch
    (test
      (call 이다 tail=- neg=1 asks=0
        (slot 가
          lit int 4
        )
        (slot -
          name 홀수
        )
      )
    )
    (body
      (exec
        (call 출력하다 tail=- neg=0 asks=0
          (slot 를
            lit str 4는 홀수가 아님\n
          )
        )
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 2가 3 이하인가: 
        (call 크다 tail=값 neg=1 asks=1
          (slot 가
            lit int 2
          )
          (slot 보다
            lit int 3
          )
        )
        lit str \n
      )
    )
  )
)
(define 큰짝수이다 predicate [가:수 보다:기준]
  (body
    (if
      (branch
        (test
          (call 이다 tail=- neg=1 asks=0
            (slot 가
              name 수
            )
            (slot -
              name 짝수
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
    (return
      (call 크다 tail=값 neg=0 asks=1
        (slot 가
          name 수
        )
        (slot 보다
          name 기준
        )
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 10이 5보다 큰 짝수: 
        (call 이다 tail=값 neg=0 asks=1
          (slot 가
            lit int 10
          )
          (slot 보다
            lit int 5
          )
          (slot -
            name 큰짝수
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
        lit str 7이 5보다 큰 짝수: 
        (call 이다 tail=값 neg=0 asks=1
          (slot 가
            lit int 7
          )
          (slot 보다
            lit int 5
          )
          (slot -
            name 큰짝수
          )
        )
        lit str \n
      )
    )
  )
)
