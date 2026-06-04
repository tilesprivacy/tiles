"""
Common utitlies used across different backends
"""

from openai_harmony import (
    Author,
    Conversation,
    DeveloperContent,
    Message,
    ReasoningEffort,
    Role,
    SystemContent,
)

from ..schemas import (
    CAssistantMessageItemParam,
    CDeveloperMessageItemParam,
    CFunctionCallItemParam,
    CFunctionCallOutputItemParam,
    CReasoningItemParam,
    CSystemMessageItemParam,
    CUserMessageItemParam,
    ResponsesRequest,
)

from openresponses_types import (
    ReasoningEffortEnum,
)

from ..reasoning_utils import ReasoningExtractor


def get_reasoning_effort(reasoning_effort_enum: ReasoningEffortEnum | None):
    reasoning_effort: ReasoningEffort
    match reasoning_effort_enum:
        case ReasoningEffortEnum.high:
            reasoning_effort = ReasoningEffort.HIGH
        case ReasoningEffortEnum.medium:
            reasoning_effort = ReasoningEffort.MEDIUM
        case ReasoningEffortEnum.low:
            reasoning_effort = ReasoningEffort.LOW
        case ReasoningEffortEnum.xhigh:
            reasoning_effort = ReasoningEffort.HIGH
        case _:
            raise TypeError("unknow reasoing effort")
    return reasoning_effort


def build_harmony_conversation(
    reasoning_effort: ReasoningEffort,
    convos: list,
):

    convo_list = [
        Message.from_role_and_content(
            Role.SYSTEM, SystemContent.new().with_reasoning_effort(reasoning_effort)
        )
    ]
    function_name = ""
    for item in convos:
        match item:
            case CUserMessageItemParam():
                content = ""
                if isinstance(item.content, list):
                    content = item.content[0].text
                else:
                    content = item.content.root
                convo_list.append(
                    Message.from_role_and_content(Role.USER, content)  # pyright: ignore
                )
            case CDeveloperMessageItemParam():
                convo_list.append(
                    Message.from_role_and_content(
                        Role.DEVELOPER,
                        DeveloperContent.new().with_instructions(
                            item.content.root
                        ),  # pyright: ignore                    )
                    )
                )
            case CAssistantMessageItemParam():
                content = ""
                if isinstance(item.content, list):
                    content = item.content[0].text
                else:
                    content = item.content.root

                convo_list.append(
                    Message.from_role_and_content(
                        Role.ASSISTANT, content
                    )  # pyright: ignore
                )
            case CSystemMessageItemParam():
                convo_list.append(
                    Message.from_role_and_content(Role.SYSTEM, item.content.root)
                )
            case CFunctionCallItemParam():
                function_name = item.name
            case CFunctionCallOutputItemParam():
                convo_list.append(
                    Message.from_author_and_content(
                        Author.new(Role.TOOL, function_name),
                        item.output,  # pyright: ignore
                    ).with_channel("commentary")
                )
            case CReasoningItemParam():
                continue
            case _:
                raise TypeError("unknown type")

    convo = Conversation.from_messages(convo_list)
    return convo


def is_harmony_family(model_name: str):
    return ReasoningExtractor.detect_model_type(model_name) == "gpt-oss"


def handle_response_input(request: ResponsesRequest):
    user_msg_item = None
    user_input_content = ""

    if isinstance(request.input, str):
        user_input_content = request.input
    else:
        user_msg_item = request.input[-1]
        if isinstance(user_msg_item, CUserMessageItemParam):
            if isinstance(user_msg_item.content, list):
                user_input_content = user_msg_item.content[0].text
            else:
                user_input_content = user_msg_item.content.root
        else:
            # FIXME: Not a user input should handle this for non-harmonic later
            user_input_content = ""
    return user_input_content


def get_tool_call_id(id: str) -> str:
    return "call_" + id.removeprefix("toolcall_")
