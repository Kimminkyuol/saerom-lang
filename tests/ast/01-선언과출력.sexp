(declare
  (target
    name 이름
  )
  (value
    lit str 새롬
  )
)
(declare
  (target
    name 나이
  )
  (value
    lit int 3
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 이름: 
        name 이름
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 나이: 
        name 나이
        lit str \n
      )
    )
  )
)
(declare
  (target
    name 나이
  )
  (value
    lit int 4
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 내년: 
        name 나이
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      lit str 가
    )
  )
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      lit str 나\n
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        name 나이
        lit str 에 10을 더하면 
        (call 더하다 tail=값 neg=0 asks=0
          (slot 에
            name 나이
          )
          (slot 를
            lit int 10
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
      lit str 중괄호: { }\n
    )
  )
)
