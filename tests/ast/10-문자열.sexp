(declare
  (target
    name 말
  )
  (value
    lit str 안녕하세요
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 글자수: 
        (field 글자수
          name 말
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
        lit str 자른 것: 
        name 칸들
        lit str   개수: 
        (field 개수
          name 칸들
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
        lit str 다시 이음: 
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
(if
  (branch
    (test
      (call 담다 tail=- neg=0 asks=0
        (slot 가
          name 말
        )
        (slot 를
          lit str 안녕
        )
      )
    )
    (body
      (exec
        (call 출력하다 tail=- neg=0 asks=0
          (slot 를
            lit str '안녕'을 담고 있음\n
          )
        )
      )
    )
  )
)
(if
  (branch
    (test
      (call 시작하다 tail=- neg=0 asks=0
        (slot 가
          name 말
        )
        (slot 로
          lit str 안
        )
      )
    )
    (body
      (exec
        (call 출력하다 tail=- neg=0 asks=0
          (slot 를
            lit str '안'으로 시작함\n
          )
        )
      )
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
      lit str 나다
      lit str 라마바
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
    (declare
      (target
        name 이름
      )
      (value
        (field 자리번째
          name 이름들
        )
      )
    )
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            name 이름
            lit str :
            (field 글자수
              name 이름
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
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      lit str 탭\t끝\n
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      lit str 따옴표 "안"\n
    )
  )
)
