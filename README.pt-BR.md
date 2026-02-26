<div align="center">
  <img src="public/icon.webp" alt="Tailscale" width="120" />
  <h1>Tailscale Tunnel Manager</h1>
  <p>Um gerenciador de túneis TCP auto-hospedado para containers Tailscale, projetado para rodar na <a href="https://discloud.com">Discloud</a>.</p>
</div>

---

## Sobre

Tailscale Tunnel Manager é uma aplicação leve que permite criar, gerenciar e monitorar túneis TCP dentro de um container conectado ao Tailscale. Ele expõe portas locais em um nó Tailscale e encaminha tráfego para hosts e portas arbitrárias usando [socat](https://linux.die.net/man/1/socat), tudo gerenciado através de uma interface web e uma API REST.

O principal caso de uso é rodar o gerenciador em um container na Discloud para que serviços implantados junto a ele (bancos de dados, ferramentas internas) se tornem acessíveis pela sua rede Tailscale sem expor nada na internet pública.

## Funcionalidades

- **Painel web** -- Crie, edite, ative/desative e exclua túneis pelo navegador. Inclui temas claro e escuro.
- **Templates de início rápido** -- Templates pré-configurados para serviços comuns como PostgreSQL, MySQL, Redis e MongoDB.
- **Teste de conexão** -- Teste a alcançabilidade do alvo diretamente pela interface antes ou depois de criar um túnel.
- **Persistência de túneis** -- A configuração dos túneis é salva em disco e restaurada automaticamente ao reiniciar o container. Túneis que falham ao restaurar são desativados em vez de tentar indefinidamente.
- **Internacionalização** -- A interface está disponível em Inglês, Português (BR), Espanhol, Francês, Alemão e Japonês.
- **Binário estaticamente linkado** -- O servidor da API é compilado para `x86_64-unknown-linux-musl`, produzindo um binário totalmente estático sem dependências de runtime.

## API REST

O servidor da API escuta na porta `3000` e serve tanto o frontend estático quanto os seguintes endpoints:

| Método | Endpoint | Descrição |
| ------ | ---------------- | ---------------------------------------- |
| GET | `/api/config` | Retorna o hostname atual do Tailscale. |
| GET | `/api/tunnels` | Lista todos os túneis com URLs de conexão. |
| POST | `/api/tunnels` | Cria um novo túnel. |
| PUT | `/api/tunnels/:id`| Atualiza um túnel existente. |
| DELETE | `/api/tunnels/:id`| Para e exclui um túnel. |
| POST | `/api/test` | Testa conectividade TCP com um host:porta. |

## Variáveis de Ambiente

| Variável | Padrão | Descrição |
| -------------------- | ----------------------- | -------------------------------------------------------- |
| `TAILSCALE_AUTHKEY` | *(obrigatório)* | Chave de autenticação do Tailscale usada para entrar na tailnet. |
| `TAILSCALE_HOSTNAME` | `tailscale-discloud` | Hostname que o nó usará na tailnet. |
| `TAILSCALE_STATE` | `/home/discloud/tailscale.state` | Caminho para o arquivo de estado do Tailscale. |
| `TUNNELS_PATH` | `/home/discloud/tunnels.json` | Caminho para o arquivo de persistência dos túneis. |

## Produção

### GitHub Releases

Cada push com tag (`v*`) aciona o workflow de release. Ele compila o binário da API para `x86_64-unknown-linux-musl` e envia um arquivo `release.zip` para o GitHub Releases contendo:

```
api          -- binário estaticamente linkado do servidor da API
start.sh     -- script de entrada que inicia o tailscaled e a API
public/      -- arquivos estáticos do frontend
```

### Dockerfile

O `Dockerfile` suporta dois modos de build controlados pelo argumento `BUILD_SOURCE`:

| `BUILD_SOURCE` | Comportamento |
| -------------- | ------------- |
| `remote` (padrao) | Baixa o `release.zip` do GitHub Releases. Nenhum arquivo local além do próprio Dockerfile é necessário. |
| `local` | Usa `api`, `start.sh` e `public/` do contexto de build via `COPY`. |

**Modo remoto** (padrão):

```
docker build -t tailscale-discloud .
```

Fixar uma versão específica ou apontar para outro repositório:

```
docker build --build-arg TUNNEL_MANAGER_VERSION=v0.1.0 -t tailscale-discloud .
docker build --build-arg GITHUB_REPO=seu-usuario/seu-fork -t tailscale-discloud .
```

**Modo local** (requer `api`, `start.sh` e `public/` no contexto de build):

```
docker build --build-arg BUILD_SOURCE=local -t tailscale-discloud .
```

A task `mise run package` produz automaticamente um diretório `dist/` com um Dockerfile modificado que usa o modo local por padrão.

### Deploy na Discloud

Para um build remoto, você precisa de apenas dois arquivos: o `Dockerfile` e um `discloud.config`. Envie-os como zip pelo painel ou CLI da Discloud. O container vai baixar todo o resto do GitHub Releases durante o build.

Para um build local, use `mise run zip` para produzir um zip completo que inclui o binario compilado, o entrypoint e os arquivos estáticos junto com o Dockerfile.

```
TYPE=bot
NAME=Tailscale
MAIN=Dockerfile
RAM=256
```

## Desenvolvimento

O desenvolvimento local usa o [mise](https://mise.jdx.dev) para gerenciar a toolchain do Rust e rodar as tasks do projeto.

### Pre-requisitos

- [mise](https://mise.jdx.dev) instalado e ativado no seu shell.
- Um linker C capaz de compilar para musl (`musl-tools` no Debian/Ubuntu).

Instalar dependências:

```
mise install
```

### Tasks Disponíveis

| Comando | Descrição |
| ---------------- | ----------------------------------------------------------- |
| `mise run build` | Compila o binário da API para `x86_64-unknown-linux-musl`. |
| `mise run package` | Roda `build`, depois monta um diretório `dist/` com o Dockerfile, binário, entrypoint, arquivos estáticos e config da Discloud. |
| `mise run zip` | Roda `package`, depois cria `dist/tailscale-discloud.zip` pronto para deploy. |
| `mise run clean` | Remove o diretório `dist/` e todos os artefatos de build do Cargo. |

O diretório `dist/` espelha a estrutura esperada pela Discloud:
```
dist/
  Dockerfile
  discloud.config
  api
  start.sh
  public/
```

### Fluxo de Trabalho

1. Faça alterações no codigo Rust em `src/` ou no frontend em `public/`.
2. Rode `mise run package` para compilar e montar tudo em `dist/`.
3. Rode `mise run zip` para produzir um zip pronto para deploy.
4. Envie `dist/release.zip` para a Discloud para testar.

## Licença

Este projeto é licenciado sob a [GNU General Public License v2.0](LICENSE). Você é livre para usar, modificar e redistribuir este software, desde que todos os trabalhos derivados permaneçam open-source sob a mesma licença e deem os devidos créditos ao autor original.

## Aviso Legal
Este projeto não e afiliado, endossado ou associado a Tailscale Inc. ou a marca Tailscale de nenhuma forma. "Tailscale" é uma marca registrada da Tailscale Inc.
