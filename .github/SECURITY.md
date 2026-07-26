# 보안 정책 / Security Policy

## 지원 버전 / Supported versions

rrdb는 아직 `0.0.x` alpha 단계입니다. 보안 수정은 `master` 브랜치에만 반영됩니다.

rrdb is still in `0.0.x` alpha. Security fixes land on `master` only.

## 취약점 신고 / Reporting a vulnerability

**공개 이슈를 열기 전에** GitHub의 비공개 신고 기능을 이용해 주세요:

[Security > Report a vulnerability](https://github.com/myyrakle/rrdb/security/advisories/new)

Please use GitHub's private reporting **before opening a public issue**.

> **참고 / Note**
>
> 이 링크는 저장소에서 private vulnerability reporting이 켜져 있을 때만 동작합니다
> (Settings → Security → Private vulnerability reporting). 링크가 404를 반환하거나
> 신고 양식이 보이지 않는다면 아직 비활성화된 상태이니, 아래 대체 경로를 이용해
> 주세요. 그 경우에도 **공개 이슈에 취약점 상세를 적지 말아 주세요.**
>
> This link only works while private vulnerability reporting is enabled for the
> repository. If it 404s or shows no report form, the feature is off — use the
> fallback below, and still **do not put vulnerability details in a public
> issue**.

### 비공개 신고가 불가능할 때 / If private reporting is unavailable

메인테이너에게 직접 연락해 주세요: [@myyrakle](https://github.com/myyrakle)
(GitHub 프로필에 공개된 연락처를 이용해 주세요.)

연락이 닿지 않고 사안이 급하지 않다면, **상세 재현 절차 없이** 이슈를 열어
비공개 연락 경로를 먼저 요청하는 방법도 있습니다.

Contact the maintainer directly via the contact details on their GitHub
profile. If that is not possible and the issue is not urgent, you may open an
issue **without reproduction details** to ask for a private channel first.

신고에 다음이 포함되면 확인이 빨라집니다:

- 재현 절차 (가능하면 SQL 질의 또는 실패하는 테스트)
- 영향 범위 — 무엇을 읽거나, 쓰거나, 망가뜨릴 수 있는지
- 확인한 커밋 해시

Including a reproduction (ideally a SQL query or a failing test), the impact,
and the commit you verified against makes triage much faster.

## 범위에 대해 / Scope

현재 rrdb는 **인증을 구현하지 않습니다.** `pgwire` 연결은 startup 메시지 직후
`AuthenticationOk`를 반환합니다. 따라서 "인증 없이 질의를 실행할 수 있다"는 것 자체는
알려진 미구현 사항이며 취약점 신고 대상이 아닙니다.

반면 **연결한 클라이언트가 데이터 디렉터리 밖으로 나가거나, 프로세스를 중단시키거나,
다른 데이터베이스의 데이터를 손상시킬 수 있는 경우**는 신고해 주세요.

rrdb currently implements **no authentication** — the pgwire handler sends
`AuthenticationOk` right after the startup message. "Queries run without
authentication" is therefore a known gap rather than a vulnerability report.

What is worth reporting is anything that lets a connected client escape the
data directory, crash the process, or corrupt data belonging to another
database.

## 신고 후 / After you report

alpha 단계 프로젝트인 만큼 정해진 응답 시한은 없습니다. 다만 신고는 확인되는 대로
회신드립니다.

This is an alpha-stage project, so there is no committed response window, but
reports are acknowledged once seen.
