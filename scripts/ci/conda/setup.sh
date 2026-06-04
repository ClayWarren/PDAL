#!/bin/bash

conda install -c conda-forge conda-build conda-index -y
pwd
ls
git clone https://github.com/conda-forge/libpdal-feedstock.git

cd libpdal-feedstock
sed -i.bak 's/"$PDAL_DRIVER_PATH"/"${PDAL_DRIVER_PATH:-}"/' recipe/scripts/activate.sh
sed -i.bak 's/"$_CONDA_SET_PDAL_DRIVER_PATH"/"${_CONDA_SET_PDAL_DRIVER_PATH:-}"/' recipe/scripts/deactivate.sh

cat > recipe/recipe_clobber.yaml <<EOL
source:
  path: ../../
  url:
  sha256:
  patches:

build:
  number: 2112
EOL

ls recipe
