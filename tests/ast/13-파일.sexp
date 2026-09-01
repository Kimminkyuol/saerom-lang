(with 기록
  (call 열다 tail=- neg=0 asks=0
    (slot 를
      lit str /tmp/새롬예제.txt
    )
  )
  (body
    (exec
      (call 쓰다 tail=- neg=0 asks=0
        (slot 를
          lit str 첫 줄\n
        )
        (slot 에
          name 기록
        )
      )
    )
    (exec
      (call 쓰다 tail=- neg=0 asks=0
        (slot 를
          lit str 둘째 줄\n
        )
        (slot 에
          name 기록
        )
      )
    )
  )
)
(with 쪽지
  (call 열다 tail=- neg=0 asks=0
    (slot 를
      lit str /tmp/새롬예제2.txt
    )
  )
  (body
    (exec
      (call 쓰다 tail=- neg=0 asks=0
        (slot 를
          lit str 가나다\n
        )
        (slot 에
          name 쪽지
        )
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 읽음:\n
        (call 읽다 tail=값 neg=0 asks=0
          (slot 를
            lit str /tmp/새롬예제.txt
          )
        )
      )
    )
  )
)
(with 손질
  (call 열다 tail=- neg=0 asks=0
    (slot 를
      lit str /tmp/새롬예제2.txt
    )
  )
  (body
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            lit str 옛 내용: 
            (call 읽다 tail=값 neg=0 asks=0
              (slot 를
                name 손질
              )
            )
          )
        )
      )
    )
    (exec
      (call 쓰다 tail=- neg=0 asks=0
        (slot 를
          lit str 새 내용\n
        )
        (slot 에
          name 손질
        )
      )
    )
  )
)
(exec
  (call 출력하다 tail=- neg=0 asks=0
    (slot 를
      (template
        lit str 바뀐 뒤: 
        (call 읽다 tail=값 neg=0 asks=0
          (slot 를
            lit str /tmp/새롬예제2.txt
          )
        )
      )
    )
  )
)
(declare
  (target
    name 글줄들
  )
  (value
    (call 자르다 tail=값 neg=0 asks=0
      (slot 를
        (call 읽다 tail=값 neg=0 asks=0
          (slot 를
            lit str /tmp/새롬예제.txt
          )
        )
      )
      (slot 로
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
      name 글줄들
    )
  )
  (body
    (declare
      (target
        name 글줄
      )
      (value
        (field 자리번째
          name 글줄들
        )
      )
    )
    (if
      (branch
        (test
          (call 이다 tail=- neg=0 asks=0
            (slot 가
              (field 글자수
                name 글줄
              )
            )
            (slot -
              lit int 0
            )
          )
        )
        (body
          continue
        )
      )
    )
    (exec
      (call 출력하다 tail=- neg=0 asks=0
        (slot 를
          (template
            name 자리
            lit str : 
            name 글줄
            lit str \n
          )
        )
      )
    )
  )
)
