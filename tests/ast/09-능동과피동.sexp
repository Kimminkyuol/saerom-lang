(declare
  (target
    name 저금통
  )
  (value
    (dict
      (entry 금액
        lit int 1000
      )
    )
  )
)
(define 저축하다 verb [에:저금통 를:돈]
  (body
    (declare
      (target
        (field 금액
          name 저금통
        )
      )
      (value
        (call 더하다 tail=값 neg=0 asks=0
          (slot 에
            (field 금액
              name 저금통
            )
          )
          (slot 를
            name 돈
          )
        )
      )
    )
  )
)
(define 저축되다 verb [에:저금통 가:돈]
  (body
    (declare
      (target
        name 새것
      )
      (value
        (field 복사본
          name 저금통
        )
      )
    )
    (exec
      (call 저축하다 tail=- neg=0 asks=0
        (slot 에
          name 새것
        )
        (slot 를
          name 돈
        )
      )
    )
    (return
      name 새것
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 피동: 
        (field 금액
          (passive 저축되다
            (head
              name 저금통
            )
            (slot 가
              lit int 500
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
        lit str 원본: 
        (field 금액
          name 저금통
        )
        lit str \n
      )
    )
  )
)
(exec
  (call 저축하다 tail=- neg=0 asks=0
    (slot 에
      name 저금통
    )
    (slot 를
      lit int 500
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 능동 뒤: 
        (field 금액
          name 저금통
        )
        lit str \n
      )
    )
  )
)
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
(exec
  (call 더하다 tail=- neg=0 asks=0
    (slot 에
      name 수들
    )
    (slot 를
      lit int 4
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 더한 뒤: 
        name 수들
        lit str \n
      )
    )
  )
)
