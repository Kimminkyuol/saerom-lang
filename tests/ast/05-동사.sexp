(define 원넓이계산하다 verb [로:반지름]
  (body
    (return
      (call 곱하다 tail=값 neg=0 asks=0
        (slot 에
          (call 곱하다 tail=값 neg=0 asks=0
            (slot 에
              name 반지름
            )
            (slot 를
              name 반지름
            )
          )
        )
        (slot 를
          lit float 3.14
        )
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 넓이: 
        (call 원넓이계산하다 tail=값 neg=0 asks=0
          (slot 로
            lit int 5
          )
        )
        lit str \n
      )
    )
  )
)
(define 전하다 verb [에게:사람 를:말]
  (body
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            name 말
            lit str , 
            name 사람
            lit str 님\n
          )
        )
      )
    )
  )
)
(exec
  (call 전하다 tail=- neg=0 asks=0
    (slot 를
      lit str 안녕
    )
    (slot 에게
      lit str 새롬
    )
  )
)
(exec
  (call 전하다 tail=- neg=0 asks=0
    (slot 에게
      lit str 새롬
    )
    (slot 를
      lit str 안녕
    )
  )
)
(define 나열하다 verb [를:값들]
  (body
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            (call 잇다 tail=값 neg=0 asks=0
              (slot 를
                name 값들
              )
            )
            lit str \n
          )
        )
      )
    )
  )
)
(define 나열하다 verb [를:값들 로:사이]
  (body
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            (call 잇다 tail=값 neg=0 asks=0
              (slot 를
                name 값들
              )
              (slot 로
                name 사이
              )
            )
            lit str \n
          )
        )
      )
    )
  )
)
(exec
  (call 나열하다 tail=- neg=0 asks=0
    (slot 를
      (list
        lit str 가
        lit str 나
      )
    )
  )
)
(exec
  (call 나열하다 tail=- neg=0 asks=0
    (slot 를
      (list
        lit str 가
        lit str 나
      )
    )
    (slot 로
      lit str -
    )
  )
)
(define 계승계산하다 verb [를:수]
  (body
    (if
      (branch
        (test
          (call 크다 tail=- neg=1 asks=0
            (slot 가
              name 수
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
        name 앞값
      )
      (value
        (call 계승계산하다 tail=값 neg=0 asks=0
          (slot 를
            (call 빼다 tail=값 neg=0 asks=0
              (slot 에서
                name 수
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
      (call 곱하다 tail=값 neg=0 asks=0
        (slot 에
          name 수
        )
        (slot 를
          name 앞값
        )
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 5! = 
        (call 계승계산하다 tail=값 neg=0 asks=0
          (slot 를
            lit int 5
          )
        )
        lit str \n
      )
    )
  )
)
(define 늘리다 verb [를:수]
  (body
    (return
      (call 더하다 tail=값 neg=0 asks=0
        (slot 에
          name 수
        )
        (slot 를
          lit int 1
        )
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        (call 늘리다 tail=값 neg=0 asks=0
          (slot 를
            lit int 9
          )
        )
        lit str \n
      )
    )
  )
)
(define 만들다 verb [로:재료 를:도시락]
  (body
    (return
      (template
        name 재료
        lit str  
        name 도시락
      )
    )
  )
)
(declare
  (target
    name 도시락
  )
  (value
    (call 만들다 tail=값 neg=0 asks=0
      (slot 로
        lit str 김
      )
      (slot 를
        lit str 주먹밥
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        name 도시락
        lit str \n
      )
    )
  )
)
(declare
  (target
    name 점수
  )
  (value
    lit int 3
  )
)
(if
  (branch
    (test
      (call 작다 tail=- neg=0 asks=0
        (slot 가
          (call 늘리다 tail=값 neg=0 asks=0
            (slot 를
              name 점수
            )
          )
        )
        (slot 보다
          lit int 5
        )
      )
    )
    (body
      (exec
        (call 출력하다 tail=- neg=0 asks=0
          (slot 를
            lit str 아직 작습니다\n
          )
        )
      )
    )
  )
)
