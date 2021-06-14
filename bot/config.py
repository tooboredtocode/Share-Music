import sys
import collections
import yaml

from pathlib import Path
from typing import Mapping, Optional

with open("config.defaults.yaml", encoding="UTF-8") as file:
    _CONFIG = yaml.safe_load(file)


def _recursive_update(defaults: dict, custom: dict):
    for key, value in defaults.items():
        if key not in custom:
            continue

        if isinstance(value, Mapping):
            if not any(isinstance(sub_value, Mapping) for sub_value in value.values()):
                defaults[key].update(custom[key])
            else:
                _recursive_update(defaults[key], custom[key])
        else:
            defaults[key] = custom[key]


if Path("config.yaml").exists():
    with open("config.yaml", encoding="UTF-8") as file:
        user_config = yaml.safe_load(file)
    _recursive_update(_CONFIG, user_config)


class _ConfigParser(type):

    def __getattr__(cls, name):
        name = name.lower()

        try:
            return _CONFIG[cls.section][name]
        except KeyError as e:
            raise AttributeError(repr(name)) from e

    def __getitem__(cls, name):
        return cls.__getattr__(name)

    def __iter__(cls):
        for name in cls.__annotations__:
            yield name, getattr(cls, name)


class Tokens(metaclass=_ConfigParser):
    section = "tokens"

    prod: str
    dev: Optional[str]
