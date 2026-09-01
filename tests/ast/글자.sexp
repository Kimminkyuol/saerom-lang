(noun 글자들 of 글
  (body
    (return
      (call 자르다 tail=값 neg=0 asks=0
        (slot 를
          name 글
        )
        (slot 로
          lit str 
        )
      )
    )
  )
)
(define 뒤집다 verb [를:글]
  (body
    (declare
      (target
        name 글자수
      )
      (value
        (field 글자수
          name 글
        )
      )
    )
    (if
      (branch
        (test
          (call 이다 tail=- neg=0 asks=0
            (slot 가
              name 글자수
            )
            (slot -
              lit int 0
            )
          )
        )
        (body
          (return
            lit str 
          )
        )
      )
    )
    (declare
      (target
        name 모은것
      )
      (value
        lit str 
      )
    )
    (loop range 자리
      (start
        lit int 1
      )
      (stop
        name 글자수
      )
      (body
        (declare
          (target
            name 뒷자리
          )
          (value
            (call 더하다 tail=값 neg=0 asks=0
              (slot 에
                (call 빼다 tail=값 neg=0 asks=0
                  (slot 에서
                    name 글자수
                  )
                  (slot 를
                    name 자리
                  )
                )
              )
              (slot 를
                lit int 1
              )
            )
          )
        )
        (declare
          (target
            name 모은것
          )
          (value
            (call 잇다 tail=값 neg=0 asks=0
              (slot 를
                (list
                  name 모은것
                  (field 뒷자리번째
                    name 글
                  )
                )
              )
            )
          )
        )
      )
    )
    (return
      name 모은것
    )
  )
)
(define 치환하다 verb [에서:글 를:옛것 로:새것]
  (body
    (return
      (call 잇다 tail=값 neg=0 asks=0
        (slot 를
          (call 자르다 tail=값 neg=0 asks=0
            (slot 를
              name 글
            )
            (slot 로
              name 옛것
            )
          )
        )
        (slot 로
          name 새것
        )
      )
    )
  )
)
(define 되풀이하다 verb [를:글 만큼:횟수]
  (body
    (if
      (branch
        (test
          (call 작다 tail=- neg=0 asks=0
            (slot 가
              name 횟수
            )
            (slot 보다
              lit int 1
            )
          )
        )
        (body
          (return
            lit str 
          )
        )
      )
    )
    (declare
      (target
        name 모은것
      )
      (value
        lit str 
      )
    )
    (loop range 번
      (start
        lit int 1
      )
      (stop
        name 횟수
      )
      (body
        (declare
          (target
            name 모은것
          )
          (value
            (call 잇다 tail=값 neg=0 asks=0
              (slot 를
                (list
                  name 모은것
                  name 글
                )
              )
            )
          )
        )
      )
    )
    (return
      name 모은것
    )
  )
)
(define 부분구하다 verb [에서:글 부터:자리 만큼:개수]
  (body
    (declare
      (target
        name 글자들
      )
      (value
        (field 글자들
          name 글
        )
      )
    )
    (declare
      (target
        name 모은것
      )
      (value
        lit str 
      )
    )
    (loop range 곳
      (start
        lit int 1
      )
      (stop
        (field 개수
          name 글자들
        )
      )
      (body
        (if
          (branch
            (test
              (call 작다 tail=- neg=0 asks=0
                (slot 가
                  name 곳
                )
                (slot 보다
                  name 자리
                )
              )
            )
            (body
              continue
            )
          )
        )
        (if
          (branch
            (test
              (call 작다 tail=- neg=1 asks=0
                (slot 가
                  name 곳
                )
                (slot 보다
                  (call 더하다 tail=값 neg=0 asks=0
                    (slot 에
                      name 자리
                    )
                    (slot 를
                      name 개수
                    )
                  )
                )
              )
            )
            (body
              break
            )
          )
        )
        (declare
          (target
            name 모은것
          )
          (value
            (call 잇다 tail=값 neg=0 asks=0
              (slot 를
                (list
                  name 모은것
                  (field 곳번째
                    name 글자들
                  )
                )
              )
            )
          )
        )
      )
    )
    (return
      name 모은것
    )
  )
)
(define 자리구하다 verb [에서:글 의:조각]
  (body
    (declare
      (target
        name 글자수
      )
      (value
        (field 글자수
          name 글
        )
      )
    )
    (declare
      (target
        name 조각수
      )
      (value
        (field 글자수
          name 조각
        )
      )
    )
    (loop range 자리
      (start
        lit int 1
      )
      (stop
        name 글자수
      )
      (body
        (if
          (branch
            (test
              (call 크다 tail=- neg=0 asks=0
                (slot 가
                  (call 더하다 tail=값 neg=0 asks=0
                    (slot 에
                      name 자리
                    )
                    (slot 를
                      name 조각수
                    )
                  )
                )
                (slot 보다
                  (call 더하다 tail=값 neg=0 asks=0
                    (slot 에
                      name 글자수
                    )
                    (slot 를
                      lit int 1
                    )
                  )
                )
              )
            )
            (body
              break
            )
          )
        )
        (declare
          (target
            name 토막
          )
          (value
            (call 부분구하다 tail=값 neg=0 asks=0
              (slot 부터
                name 자리
              )
              (slot 에서
                name 글
              )
              (slot 만큼
                name 조각수
              )
            )
          )
        )
        (if
          (branch
            (test
              (call 같다 tail=- neg=0 asks=0
                (slot 가
                  name 토막
                )
                (slot 와
                  name 조각
                )
              )
            )
            (body
              (return
                name 자리
              )
            )
          )
        )
      )
    )
    (return
      lit int 0
    )
  )
)
(define 왼쪽채움하다 verb [를:글 만큼:너비]
  (body
    (declare
      (target
        name 모자람
      )
      (value
        (call 빼다 tail=값 neg=0 asks=0
          (slot 에서
            name 너비
          )
          (slot 를
            (field 글자수
              name 글
            )
          )
        )
      )
    )
    (if
      (branch
        (test
          (call 작다 tail=- neg=0 asks=0
            (slot 가
              name 모자람
            )
            (slot 보다
              lit int 1
            )
          )
        )
        (body
          (return
            name 글
          )
        )
      )
    )
    (return
      (call 잇다 tail=값 neg=0 asks=0
        (slot 를
          (list
            (call 되풀이하다 tail=값 neg=0 asks=0
              (slot 를
                lit str  
              )
              (slot 만큼
                name 모자람
              )
            )
            name 글
          )
        )
      )
    )
  )
)
