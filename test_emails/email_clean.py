import csv
from collections import namedtuple
from typing import Optional

Email = namedtuple(
    'Email',
    ['from_email', 'to_email', 'from_name', 'to_name', 'content', 'message_id']
)


def strip_problems(x: str) -> str:
    return x.replace(',', '').replace('"', '').replace("'", '').replace('\n', '').strip()


def strip_problems_and_brackets(x: str) -> str:
    stripped: str = strip_problems(x)
    no_brackets = ""

    ignoring = False

    for char in stripped:
        if char == "<":
            ignoring = True
        if char == ">":
            ignoring = False
            continue

        if not ignoring:
            no_brackets += char

    return no_brackets.replace(' @ ENRON', '').strip()


def read_email(email: str) -> Optional[Email]:
    from_email = None
    to_email = None
    from_name = None
    to_name = None
    content = None
    message_id = None

    lines = email.splitlines()

    for index, line in enumerate(lines):

        if line[0:11] == "Message-ID:":
            message_id = strip_problems(line[12:])
        if line[0:5] == "From:":
            from_email = strip_problems_and_brackets(line[5:].strip())
        if line[0:3] == "To:":
            to_email = strip_problems_and_brackets(line[3:].strip())
        if line[0:5] == "X-To:":
            to_name = strip_problems_and_brackets(line[5:].strip())
        if line[0:7] == "X-From:":
            from_name = strip_problems_and_brackets(line[7:].strip())
        if line[0:11] == "X-FileName:":
            raw_content = ' '.join(lines[index + 1:])
            content = strip_problems(raw_content)
            break

    # or content == None or message_id == None:
    if from_email == None or to_email == None or from_name == None or to_name == None:
        return None

    return Email(from_email, to_email, from_name, to_name, content, message_id)


def read_emails_from_file(filename: str) -> list[Email]:
    emails = []

    with open(filename, "r") as file:
        csvFile = csv.reader(file)

        for i, line in enumerate(csvFile):
            # Skip the header
            if i == 0:
                continue

            email = read_email(line[2])

            if email is not None:
                emails.append(email)

    return emails


def write_emails_to_csv(emails: list[Email], filename: str):
    with open(filename, "w") as file:
        file.write(
            'from_email, to_email, from_name, to_name, content, message_id,\n')

        for email in emails:
            file.write(
                f'{email.from_email}, {email.to_email}, {email.from_name}, {email.to_name}, {email.content}, {email.message_id},\n')


FILENAME = "split_emails.csv"
CLEANED_FILENAME = "cleaned_emails.csv"
emails = read_emails_from_file(FILENAME)
write_emails_to_csv(emails, CLEANED_FILENAME)
