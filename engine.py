#!/usr/bin/env python
import requests
import micropickle
from config import get_cpm_token


def get_answers(code: str):
    if len(code) > 10:
        code_start = code.find('=')
        code = code[code_start + 1:]
    response = requests.get(f"https://api.matetech.ru/api/public/companies/3/test_attempts/{code}/result",
                            headers={"Authorization": get_cpm_token()})
    print(get_cpm_token())
    resp = response.json()
    if response.status_code == 404:
        return {'message': 'Перепроверь айди', 'channel': False}
    response_course = requests.get(
        f"https://api.matetech.ru/api/public/companies/3/courses/{resp['data']['test_lesson']['course_id']}?with"
        f"=paymentPackagesBySort;advantagesBySort;subcategories_with_categories;availablePackagesBySort;locals"
        f"&append_avg_rating=true&append_approved_reviews_has_points_count=true&withCount=students,lessons",
        headers={"Authorization": get_cpm_token()})
    result = ""
    resp_c = response_course.json()
    course = resp_c['data']['name']
    result += course + '\n'
    number = 0
    lesson_name = resp['data']['test_lesson']['name']
    result += lesson_name + '\n'
    cl = 0
    if course.find("8 кл") != -1:
        cl = 8
    if course.find("9 кл") != -1:
        cl = 9
    if course.find("10 кл") != -1:
        cl = 10
    if course.find("11 кл") != -1:
        cl = 11
    print(cl)
    for question in resp['data']['questions'][0]:
        number += 1
        pr = f"№{number}: "
        if len(question['answers']) == 1:
            pr += question['answers'][0]['value']
        else:
            for answer in question['answers']:
                if answer['correct']:
                    pr += str(answer['sort'])
        result += pr + '\n'
    sent_tests = micropickle.load_obj("tests")
    try:
        sent_tests.index(resp['data']['test_lesson']['id'])
    except ValueError:
        sent_tests.append(resp['data']['test_lesson']['id'])
        print(sent_tests)
        micropickle.save_obj(sent_tests, "tests")
        return {'message': result, 'channel': True, 'cl': cl}
    return {'message': result, 'channel': False}
