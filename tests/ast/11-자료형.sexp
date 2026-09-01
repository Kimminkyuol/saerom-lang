(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 12의 자료형: 
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
        lit str 3.5의 자료형: 
        (field 자료형
          lit float 3.5
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
        lit str "가"의 자료형: 
        (field 자료형
          lit str 가
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
        lit str [1]의 자료형: 
        (field 자료형
          (list
            lit int 1
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
        lit str 참의 자료형: 
        (field 자료형
          lit bool 참
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
        lit str "12" → 정수: 
        (call 더하다 tail=값 neg=0 asks=0
          (slot 에
            (call 바꾸다 tail=값 neg=0 asks=0
              (slot 를
                lit str 12
              )
              (slot 로
                name 정수
              )
            )
          )
          (slot 를
            lit int 1
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
        lit str "3.5" → 실수: 
        (call 바꾸다 tail=값 neg=0 asks=0
          (slot 를
            lit str 3.5
          )
          (slot 로
            name 실수
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
        lit str 12 → 문자열: 
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
        lit str "참" → 논리값: 
        (call 바꾸다 tail=값 neg=0 asks=0
          (slot 를
            lit str 참
          )
          (slot 로
            name 논리값
          )
        )
        lit str \n
      )
    )
  )
)
(declare
  (target
    name 값
  )
  (value
    lit int 12
  )
)
(if
  (branch
    (test
      (call 이다 tail=- neg=0 asks=0
        (slot 가
          (field 자료형
            name 값
          )
        )
        (slot -
          name 정수
        )
      )
    )
    (body
      (exec
        (call 출력하다 tail=- neg=0 asks=0
          (slot 를
            lit str 정수다\n
          )
        )
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
        lit str 가
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 철수의 자료형: 
        (field 자료형
          name 철수
        )
        lit str \n
      )
    )
  )
)
