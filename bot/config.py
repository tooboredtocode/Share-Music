import sys
import collections
import yaml

from pathlib import Path
from typing import Mapping, Optional

from bot import root_path

if not Path(default_config_path := root_path / "config.defaults.yaml").exists():
    raise FileNotFoundError(
        f"Cannot find default config! If you accidentally moved it, place it here: {default_config_path}"
    )

with open(default_config_path, encoding="UTF-8") as file:
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
        elif isinstance(value, list) and "!!include_default" in custom[key]:
            defaults[key].extend(custom[key])
            defaults[key].remove("!!include_default")
        else:
            defaults[key] = custom[key]


if Path(user_config_path := root_path / "config.yaml").exists():
    with open(user_config_path, encoding="UTF-8") as file:
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


class _ListConfigParser(type):

    def __len__(cls):
        return len(_CONFIG[cls.section])

    def __getattr__(cls, index: int):
        if index == "cls":
            return None
        elif cls.cls:
            return cls.cls(**_CONFIG[cls.section][index])
        else:
            return _CONFIG[cls.section][index]

    def __getitem__(cls, index: int):
        return cls.__getattr__(index)

    def __iter__(cls):
        for index in range(cls.__len__()):
            yield cls.__getattr__(index)


class Tokens(metaclass=_ConfigParser):
    section = "tokens"

    prod: str
    dev: Optional[str]


class LoggingConfig(collections.abc.Mapping):

    def __init__(self, **kwargs):
        for key, value in kwargs.items():
            if key == "sink":
                if value == "sys.stdout":
                    self.sink = sys.stdout
                    continue

                self.sink = root_path / "logs" / value
                continue

            setattr(self, key, value)

    def __getitem__(self, key):
        return getattr(self, key)

    def __iter__(self):
        for key in self.__dict__.keys():
            yield key

    def __len__(self):
        return len(self.__dict__)


class LoggingConfigs(metaclass=_ListConfigParser):
    section = "logging"
    cls = LoggingConfig
