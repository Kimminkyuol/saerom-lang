import 수학 [제곱근]
(noun 합 of 수들
  (body
    (if
      (branch
        (test
          (call 이다 tail=- neg=0 asks=0
            (slot 가
              (field 개수
                name 수들
              )
            )
            (slot -
              lit int 0
            )
          )
        )
        (body
          (return
            lit int 0
          )
        )
      )
    )
    (declare
      (target
        name 모은것
      )
      (value
        lit int 0
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
            name 모은것
          )
          (value
            (call 더하다 tail=값 neg=0 asks=0
              (slot 에
                name 모은것
              )
              (slot 를
                (field 자리번째
                  name 수들
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
(noun 평균 of 수들
  (body
    (declare
      (target
        name 개수
      )
      (value
        (field 개수
          name 수들
        )
      )
    )
    (if
      (branch
        (test
          (call 이다 tail=- neg=0 asks=0
            (slot 가
              name 개수
            )
            (slot -
              lit int 0
            )
          )
        )
        (body
          (return
            lit int 0
          )
        )
      )
    )
    (return
      (call 나누다 tail=값 neg=0 asks=0
        (slot 를
          (field 합
            name 수들
          )
        )
        (slot 로
          name 개수
        )
      )
    )
  )
)
(noun 최댓값 of 수들
  (body
    (if
      (branch
        (test
          (call 이다 tail=- neg=0 asks=0
            (slot 가
              (field 개수
                name 수들
              )
            )
            (slot -
              lit int 0
            )
          )
        )
        (body
          (exec
            (call 종료하다 tail=- neg=0 asks=0
              (slot 로
                lit str 빈 목록에는 최댓값이 없음
              )
            )
          )
        )
      )
    )
    (declare
      (target
        name 으뜸값
      )
      (value
        (field 첫째
          name 수들
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
        (if
          (branch
            (test
              (call 크다 tail=- neg=0 asks=0
                (slot 가
                  (field 자리번째
                    name 수들
                  )
                )
                (slot 보다
                  name 으뜸값
                )
              )
            )
            (body
              (declare
                (target
                  name 으뜸값
                )
                (value
                  (field 자리번째
                    name 수들
                  )
                )
              )
            )
          )
        )
      )
    )
    (return
      name 으뜸값
    )
  )
)
(noun 최솟값 of 수들
  (body
    (if
      (branch
        (test
          (call 이다 tail=- neg=0 asks=0
            (slot 가
              (field 개수
                name 수들
              )
            )
            (slot -
              lit int 0
            )
          )
        )
        (body
          (exec
            (call 종료하다 tail=- neg=0 asks=0
              (slot 로
                lit str 빈 목록에는 최솟값이 없음
              )
            )
          )
        )
      )
    )
    (declare
      (target
        name 으뜸값
      )
      (value
        (field 첫째
          name 수들
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
        (if
          (branch
            (test
              (call 작다 tail=- neg=0 asks=0
                (slot 가
                  (field 자리번째
                    name 수들
                  )
                )
                (slot 보다
                  name 으뜸값
                )
              )
            )
            (body
              (declare
                (target
                  name 으뜸값
                )
                (value
                  (field 자리번째
                    name 수들
                  )
                )
              )
            )
          )
        )
      )
    )
    (return
      name 으뜸값
    )
  )
)
(noun 범위 of 수들
  (body
    (return
      (call 빼다 tail=값 neg=0 asks=0
        (slot 에서
          (field 최댓값
            name 수들
          )
        )
        (slot 를
          (field 최솟값
            name 수들
          )
        )
      )
    )
  )
)
(noun 분산 of 수들
  (body
    (declare
      (target
        name 개수
      )
      (value
        (field 개수
          name 수들
        )
      )
    )
    (if
      (branch
        (test
          (call 이다 tail=- neg=0 asks=0
            (slot 가
              name 개수
            )
            (slot -
              lit int 0
            )
          )
        )
        (body
          (return
            lit int 0
          )
        )
      )
    )
    (declare
      (target
        name 평균
      )
      (value
        (field 평균
          name 수들
        )
      )
    )
    (declare
      (target
        name 모은것
      )
      (value
        lit int 0
      )
    )
    (loop range 자리
      (start
        lit int 1
      )
      (stop
        name 개수
      )
      (body
        (declare
          (target
            name 차이
          )
          (value
            (call 빼다 tail=값 neg=0 asks=0
              (slot 에서
                (field 자리번째
                  name 수들
                )
              )
              (slot 를
                name 평균
              )
            )
          )
        )
        (declare
          (target
            name 제곱
          )
          (value
            (call 곱하다 tail=값 neg=0 asks=0
              (slot 에
                name 차이
              )
              (slot 를
                name 차이
              )
            )
          )
        )
        (declare
          (target
            name 모은것
          )
          (value
            (call 더하다 tail=값 neg=0 asks=0
              (slot 에
                name 모은것
              )
              (slot 를
                name 제곱
              )
            )
          )
        )
      )
    )
    (return
      (call 나누다 tail=값 neg=0 asks=0
        (slot 를
          name 모은것
        )
        (slot 로
          name 개수
        )
      )
    )
  )
)
(noun 표준편차 of 수들
  (body
    (return
      (field 제곱근
        (field 분산
          name 수들
        )
      )
    )
  )
)
