(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      lit str 이름? 
    )
  )
)
(declare
  (target
    name 이름
  )
  (value
    (call 입력받다 tail=값 neg=0 asks=0
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      lit str 나이? 
    )
  )
)
(declare
  (target
    name 나이
  )
  (value
    (call 바꾸다 tail=값 neg=0 asks=0
      (slot 를
        (call 입력받다 tail=값 neg=0 asks=0
        )
      )
      (slot 로
        name 정수
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        name 이름
        lit str 님, 내년에 
        (call 더하다 tail=값 neg=0 asks=0
          (slot 에
            name 나이
          )
          (slot 를
            lit int 1
          )
        )
        lit str 살\n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      lit str 남은 줄:\n
    )
  )
)
(loop range 자리
  (start
    lit int 1
  )
  (stop
    lit int 2
  )
  (body
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            lit str   
            name 자리
            lit str : 
            (call 입력받다 tail=값 neg=0 asks=0
            )
            lit str \n
          )
        )
      )
    )
  )
)
