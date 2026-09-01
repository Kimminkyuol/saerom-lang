import 통계 [평균]
(declare
  (target
    name 철수
  )
  (value
    (dict
      (entry 이름
        lit str 김철수
      )
      (entry 나이
        lit int 17
      )
      (entry 점수들
        (list
          lit int 88
          lit int 92
          lit int 79
        )
      )
    )
  )
)
(declare
  (target
    name 영희
  )
  (value
    (dict
    )
  )
)
(declare
  (target
    (field 이름
      name 영희
    )
  )
  (value
    lit str 이영희
  )
)
(declare
  (target
    (field 나이
      name 영희
    )
  )
  (value
    lit int 18
  )
)
(declare
  (target
    (field 점수들
      name 영희
    )
  )
  (value
    (list
      lit int 95
      lit int 91
      lit int 100
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        (field 이름
          name 철수
        )
        lit str , 
        (field 나이
          name 철수
        )
        lit str 살\n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        name 영희
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 자료형: 
        (field 자료형
          name 철수
        )
        lit str \n
      )
    )
  )
)
(declare
  (target
    (field 나이
      name 철수
    )
  )
  (value
    lit int 18
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 한 살 더: 
        (field 나이
          name 철수
        )
        lit str \n
      )
    )
  )
)
(noun 평균점수 of 학생
  (body
    (return
      (field 평균
        (field 점수들
          name 학생
        )
      )
    )
  )
)
(define 우등생이다 predicate [가:학생]
  (body
    (return
      (call 작다 tail=값 neg=1 asks=1
        (slot 가
          (field 평균점수
            name 학생
          )
        )
        (slot 보다
          lit int 90
        )
      )
    )
  )
)
(declare
  (target
    name 학생들
  )
  (value
    (list
      name 철수
      name 영희
    )
  )
)
(loop range 자리
  (start
    lit int 1
  )
  (stop
    (field 개수
      name 학생들
    )
  )
  (body
    (declare
      (target
        name 학생
      )
      (value
        (field 자리번째
          name 학생들
        )
      )
    )
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            (field 이름
              name 학생
            )
            lit str \t
            (field 평균점수
              name 학생
            )
            lit str \t
            (call 이다 tail=값 neg=0 asks=1
              (slot 가
                name 학생
              )
              (slot -
                name 우등생
              )
            )
            lit str \n
          )
        )
      )
    )
  )
)
