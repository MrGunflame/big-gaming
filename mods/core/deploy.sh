#!/bin/bash

cp ./build/core.mod ../../game_server/mods/core.mod
cp ./build/core.mod ../../game_client/mods/core.mod

cp ./build/scripts/* ../../game_server/scripts/
cp ./build/scripts/* ../../game_client/scripts/

cp ./models/* ../../game_server/assets/
cp ./models/* ../../game_client/assets/
