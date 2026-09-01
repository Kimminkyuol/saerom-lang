import 수학 [짝수이다]
(loop range 수
  (start
    lit int 1
  )
  (stop
    lit int 5
  )
  (body
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            name 수
            lit str  
          )
        )
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      lit str \n
    )
  )
)
(loop range 수
  (start
    lit int 3
  )
  (stop
    lit int 1
  )
  (body
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            name 수
            lit str  
          )
        )
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      lit str \n
    )
  )
)
(loop range 수
  (start
    lit int 10
  )
  (stop
    lit int 0
  )
  (step
    lit int 5
  )
  (body
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            name 수
            lit str  
          )
        )
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      lit str \n
    )
  )
)
(loop range 수
  (start
    lit int 0
  )
  (stop
    lit int 10
  )
  (step
    lit int 5
  )
  (body
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            name 수
            lit str  
          )
        )
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      lit str \n
    )
  )
)
(declare
  (target
    name 이름들
  )
  (value
    (list
      lit str 가
      lit str 나
      lit str 다
    )
  )
)
(loop range 자리
  (start
    lit int 1
  )
  (stop
    (field 개수
      name 이름들
    )
  )
  (body
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            name 자리
            lit str :
            (field 자리번째
              name 이름들
            )
            lit str  
          )
        )
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      lit str \n
    )
  )
)
(loop range 수
  (start
    lit int 1
  )
  (stop
    lit int 10
  )
  (body
    (if
      (branch
        (test
          (call 이다 tail=- neg=0 asks=0
            (slot 가
              name 수
            )
            (slot -
              name 짝수
            )
          )
        )
        (body
          continue
        )
      )
    )
    (if
      (branch
        (test
          (call 크다 tail=- neg=0 asks=0
            (slot 가
              name 수
            )
            (slot 보다
              lit int 7
            )
          )
        )
        (body
          break
        )
      )
    )
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            name 수
            lit str  
          )
        )
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      lit str \n
    )
  )
)
(declare
  (target
    name 남은것
  )
  (value
    lit int 3
  )
)
(loop while
  (test
    (call 크다 tail=값 neg=0 asks=0
      (slot 가
        name 남은것
      )
      (slot 보다
        lit int 0
      )
    )
  )
  (body
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            name 남은것
            lit str  
          )
        )
      )
    )
    (declare
      (target
        name 남은것
      )
      (value
        (call 빼다 tail=값 neg=0 asks=0
          (slot 에서
            name 남은것
          )
          (slot 를
            lit int 1
          )
        )
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      lit str \n
    )
  )
)
