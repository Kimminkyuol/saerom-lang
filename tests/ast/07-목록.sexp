import 통계 [합 평균 최댓값]
(declare
  (target
    name 수들
  )
  (value
    (list
      lit int 3
      lit int 8
      lit int 15
      lit int 4
      lit int 23
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 개수: 
        (field 개수
          name 수들
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
        lit str 첫째: 
        (field 첫째
          name 수들
        )
        lit str   마지막: 
        (field 마지막
          name 수들
        )
        lit str   3번째: 
        (field 3번째
          name 수들
        )
        lit str \n
      )
    )
  )
)
(declare
  (target
    name 자리
  )
  (value
    lit int 2
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        name 자리
        lit str 번째: 
        (field 자리번째
          name 수들
        )
        lit str \n
      )
    )
  )
)
(loop range 자리
  (start
    lit int 1
  )
  (stop
    (field 개수
      name 수들
    )
  )
  (body
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            (field 자리번째
              name 수들
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
(declare
  (target
    name 큰수들
  )
  (value
    (list
    )
  )
)
(loop range 자리
  (start
    lit int 1
  )
  (stop
    (field 개수
      name 수들
    )
  )
  (body
    (declare
      (target
        name 수
      )
      (value
        (field 자리번째
          name 수들
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
              lit int 10
            )
          )
        )
        (body
          (exec
            (call 더하다 tail=- neg=0 asks=0
              (slot 에
                name 큰수들
              )
              (slot 를
                name 수
              )
            )
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
        lit str 10보다 큼: 
        name 큰수들
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 합: 
        (field 합
          name 수들
        )
        lit str   평균: 
        (field 평균
          name 수들
        )
        lit str   최댓값: 
        (field 최댓값
          name 수들
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
        lit str 복사본: 
        (field 복사본
          name 수들
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
        lit str 담기: 
        (call 담다 tail=값 neg=0 asks=1
          (slot 가
            name 수들
          )
          (slot 를
            lit int 15
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
        lit str 자료형: 
        (field 자료형
          name 수들
        )
        lit str \n
      )
    )
  )
)
(declare
  (target
    name 칸들
  )
  (value
    (call 자르다 tail=값 neg=0 asks=0
      (slot 를
        lit str 가,나,다
      )
      (slot 로
        lit str ,
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 자르기: 
        name 칸들
        lit str \n
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 잇기: 
        (call 잇다 tail=값 neg=0 asks=0
          (slot 를
            name 칸들
          )
          (slot 로
            lit str -
          )
        )
        lit str \n
      )
    )
  )
)
