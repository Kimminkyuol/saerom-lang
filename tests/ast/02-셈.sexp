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
        lit str 3^2 = 
        (call 곱하다 tail=값 neg=0 asks=0
          (slot 에
            lit int 3
          )
          (slot 를
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
        lit str (2+3)*4 = 
        (call 곱하다 tail=값 neg=0 asks=0
          (slot 에
            (call 더하다 tail=값 neg=0 asks=0
              (slot 에
                lit int 2
              )
              (slot 를
                lit int 3
              )
            )
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
        lit str -5의 절댓값 = 
        (call 빼다 tail=값 neg=0 asks=0
          (slot 에서
            lit int 0
          )
          (slot 를
            lit int -5
          )
        )
        lit str \n
      )
    )
  )
)
