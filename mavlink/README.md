# MAVLink
```bash
git clone git@gitlab.smart-glow.ru:rostelecom_remote/mavlink.git --recursive
```

Форк MAVLink с дополнительным диалектом rtc_rc, реализующим связь с пультом управления.  
Диалект rtc_rc включен в диалект ardupilotmega

## Генерация
Для генерации необходим python3 < 3.12 (в 12 версии удалили устаревший модуль `imp`)

### С/C++
1. Установите необходимые зависимости
    ```bash
    python -m pip install -r ./pymavlink/requirements.txt
    ```
2. Запустите генератор
    ```bash
    python -m mavgenerate
    ```
3. `XML`: Необходимо выбрать диалект в директории ./message_definitions/v1.0  
    В нашем случае **./message_definitions/v1.0/rt_rc.xml**

    `Out`: Название директории, где будет сгенерирована библиотека.  
    **you_project/lib/mavlink**

    `Language`: **C**/**C++** 

    `Protocol`: 2.0

    `Validate` и `Validate Units` на свое усмотрение

[Пример работы с библиотекой](https://gitlab.smart-glow.ru/uav-development/ardumav)

### Python
Полной инструкции я не нашел, здесь опишу как была сделана библиотека для проекта [mav-gc](https://gitlab.smart-glow.ru/rostelecom_remote/mav-gc)

Необходимо сгенерировать два файла.  
Шаги 1-2 аналогичны С/C++

3. `Out`: Не название директории, а название выходного файла.  
    В нашем случае **temp1/rt_rc** и **temp2/rt_rc**

    `Language`: Python3

    `Protocol`: Генерируем один файл для **1.0** и один для **2.0**
4. Файл, сгенерированный для протокола 1.0 копируем в **./pymavlink/dialects/v10**  
    Файл, сгенерированный для протокола 2.0 копируем в **./pymavlink/dialects/v20**
5. Всю директорию **./pymavlink** копируем в необходимый проект

ВАЖНО: при копировании библиотеки pymavlink в свой проект закомментируйте строку `*.py` в **./pymavlink./dialects/gitignore**

[Пример работы с библиотекой](https://gitlab.smart-glow.ru/rostelecom_remote/mav-gc)