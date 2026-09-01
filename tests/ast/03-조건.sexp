(declare
  (target
    name 나이
  )
  (value
    lit int 20
  )
)
(if
  (branch
    (test
      (call 크다 tail=- neg=0 asks=0
        (slot 가
          name 나이
        )
        (slot 보다
          lit int 20
        )
      )
    )
    (body
      (exec
        (call 출력하다 tail=- neg=0 asks=0
          (slot 를
            lit str 어른\n
          )
        )
      )
    )
  )
  (branch
    (test
      (call 이다 tail=- neg=0 asks=0
        (slot 가
          name 나이
        )
        (slot -
          lit int 20
        )
      )
    )
    (body
      (exec
        (call 출력하다 tail=- neg=0 asks=0
          (slot 를
            lit str 갓 어른\n
          )
        )
      )
    )
  )
  (otherwise
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          lit str 아이\n
        )
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
              name 나이
            )
            (slot 보다
              lit int 10
            )
          )
        )
        (slot -
          (call 작다 tail=- neg=0 asks=0
            (slot 가
              name 나이
            )
            (slot 보다
              lit int 30
            )
          )
        )
      )
    )
    (body
      (exec
        (call 출력하다 tail=- neg=0 asks=0
          (slot 를
            lit str 20대쯤\n
          )
        )
      )
    )
  )
)
(if
  (branch
    (test
      (call 또는 tail=- neg=0 asks=0
        (slot -
          (call 이다 tail=- neg=0 asks=0
            (slot 가
              name 나이
            )
            (slot -
              lit int 100
            )
          )
        )
        (slot -
          (call 이다 tail=- neg=0 asks=0
            (slot 가
              name 나이
            )
            (slot -
              lit int 20
            )
          )
        )
      )
    )
    (body
      (exec
        (call 출력하다 tail=- neg=0 asks=0
          (slot 를
            lit str 스물이거나 백\n
          )
        )
      )
    )
  )
)
(if
  (branch
    (test
      (call 크다 tail=- neg=1 asks=0
        (slot 가
          name 나이
        )
        (slot 보다
          lit int 30
        )
      )
    )
    (body
      (exec
        (call 출력하다 tail=- neg=0 asks=0
          (slot 를
            lit str 서른 이하\n
          )
        )
      )
    )
  )
)
(if
  (branch
    (test
      (call 이다 tail=- neg=1 asks=0
        (slot 가
          name 나이
        )
        (slot -
          lit int 19
        )
      )
    )
    (body
      (exec
        (call 출력하다 tail=- neg=0 asks=0
          (slot 를
            lit str 열아홉이 아님\n
          )
        )
      )
    )
  )
)
(if
  (branch
    (test
      (call 또는 tail=- neg=0 asks=0
        (slot -
          (call 작다 tail=- neg=0 asks=0
            (slot 가
              name 나이
            )
            (slot 보다
              lit int 20
            )
          )
        )
        (slot -
          (call 같다 tail=- neg=0 asks=0
            (slot 가
              name 나이
            )
            (slot 와
              lit int 20
            )
          )
        )
      )
    )
    (body
      (exec
        (call 출력하다 tail=- neg=0 asks=0
          (slot 를
            lit str 스물 이하\n
          )
        )
      )
    )
  )
)
(if
  (branch
    (test
      (call 작다 tail=- neg=1 asks=0
        (slot 가
          name 나이
        )
        (slot 보다
          lit int 20
        )
      )
    )
    (body
      (exec
        (call 출력하다 tail=- neg=0 asks=0
          (slot 를
            lit str 스무 살부터\n
          )
        )
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
              name 나이
            )
            (slot 보다
              lit int 19
            )
          )
        )
        (slot -
          (call 작다 tail=- neg=0 asks=0
            (slot 가
              name 나이
            )
            (slot 보다
              lit int 30
            )
          )
        )
      )
    )
    (body
      (exec
        (call 출력하다 tail=- neg=0 asks=0
          (slot 를
            lit str 스물에서 스물아홉\n
          )
        )
      )
    )
  )
)
(if
  (branch
    (test
      (call 크다 tail=- neg=1 asks=0
        (slot 가
          name 나이
        )
        (slot 보다
          lit int 20
        )
      )
    )
    (body
      (exec
        (call 출력하다 tail=- neg=0 asks=0
          (slot 를
            lit str 스무 살까지\n
          )
        )
      )
    )
  )
)
