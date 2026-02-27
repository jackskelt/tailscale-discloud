<div align="center">
  <img src="public/icon.webp" alt="Tailscale" width="120" />
  <h1>Tailscale Tunnel Manager</h1>
  <p>Um gerenciador de túneis TCP para containers Tailscale, projetado para rodar na <a href="https://discloud.com">Discloud</a>.</p>
</div>

<table>
  <tr>
    <td width="50%">
      <img src="images/banner.png" alt="Banner — Painel web do Tailscale Tunnel Manager" width="100%" />
    </td>
    <td width="50%">
      <img src="images/deploy/tailscale-diagram.png" alt="Arquitetura — Como o seu computador se conecta aos serviços na Discloud através da rede Tailscale" width="100%" />
    </td>
  </tr>
</table>

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

## Deploy

### Sumário

1. [Configurando o Tailscale](#1-configurando-o-tailscale)
   1. [Criar uma conta no Tailscale](#11-criar-uma-conta-no-tailscale)
   2. [Instalar o cliente Tailscale](#12-instalar-o-cliente-tailscale)
   3. [Conectar e verificar](#13-conectar-e-verificar)
2. [Deploy na Discloud](#2-deploy-na-discloud)
   1. [Baixar o zip de deploy](#21-baixar-o-zip-de-deploy)
   2. [Acessar a dashboard da Discloud](#22-acessar-a-dashboard-da-discloud)
   3. [Fazer upload do zip](#23-fazer-upload-do-zip)
   4. [Encontrar o link de login do Tailscale](#24-encontrar-o-link-de-login-do-tailscale)
   5. [Autorizar o nó](#25-autorizar-o-nó)
   6. [Verificar a máquina no Tailscale](#26-verificar-a-máquina-no-tailscale)
   7. [Ativar a VLAN na Discloud](#27-ativar-a-vlan-na-discloud)
   8. [Acessar o painel web](#28-acessar-o-painel-web)
3. [Utilização — Criando seu primeiro túnel](#3-utilização--criando-seu-primeiro-túnel)
   1. [Hospedar uma instância MySQL a partir de um template](#31-hospedar-uma-instância-mysql-a-partir-de-um-template)
   2. [Configurar a VLAN na aplicação MySQL](#32-configurar-a-vlan-na-aplicação-mysql)
   3. [Abrir o painel do Tunnel Manager](#33-abrir-o-painel-do-tunnel-manager)
   4. [Criar um novo túnel](#34-criar-um-novo-túnel)
   5. [Verificar se o túnel está ativo](#35-verificar-se-o-túnel-está-ativo)
   6. [Obter a string de conexão](#36-obter-a-string-de-conexão)
   7. [Conectar a partir da sua máquina local](#37-conectar-a-partir-da-sua-máquina-local)

---

### 1. Configurando o Tailscale

#### 1.1 Criar uma conta no Tailscale

Acesse [https://tailscale.com](https://tailscale.com) e crie uma conta gratuita. Você pode se cadastrar com Google, Microsoft, GitHub ou outros provedores de identidade.

![Página de cadastro do Tailscale mostrando os provedores de identidade disponíveis](images/deploy/tailscale-signup.png)

#### 1.2 Instalar o cliente Tailscale

Instale o cliente Tailscale na máquina que você deseja usar para acessar seus túneis (seu notebook, desktop, etc.).

O Tailscale suporta Windows, macOS, Linux, iOS e Android. Siga o guia oficial de instalação para a sua plataforma:

📖 **[Downloads e Guia de Instalação do Tailscale](https://tailscale.com/download)**

![Página de download do Tailscale mostrando clientes para diferentes plataformas](images/deploy/tailscale-download.png)

#### 1.3 Conectar e verificar

Após instalar, abra o cliente Tailscale e faça login com a mesma conta que você criou no passo 1.1.

Uma vez conectado, sua máquina deve aparecer no console de administração do Tailscale em [https://login.tailscale.com/admin/machines](https://login.tailscale.com/admin/machines).

![Console de administração do Tailscale mostrando a máquina local listada em Machines](images/deploy/tailscale-machines-local.png)

---

### 2. Deploy na Discloud

#### 2.1 Baixar o zip de deploy

Acesse a página de [GitHub Releases](https://github.com/jackskelt/tailscale-discloud/releases) e baixe um dos zips de deploy:

- **`deploy-remote.zip`** — Contém apenas o `Dockerfile` e o `discloud.config`. O container baixa o binário do GitHub Releases durante o build.
- **`deploy-static.zip`** — Contém o binário compilado, entrypoint, arquivos estáticos, `Dockerfile` e `discloud.config`.

Ambos os zips seguem exatamente os mesmos passos de deploy abaixo. A única diferença é como a imagem Docker é construída internamente.

> **💡 Dica sobre atualizações:** Se você usar o `deploy-remote.zip`, seu container sempre baixará a **última** release do GitHub quando for reconstruído. Com o `deploy-static.zip`, o binário está embutido no zip, então você precisa baixar um novo zip das Releases e reenviá-lo para atualizar.

#### 2.2 Acessar a dashboard da Discloud

Faça login na dashboard da Discloud em [https://discloud.com/dashboard](https://discloud.com/dashboard).

![Página principal da dashboard da Discloud](images/deploy/discloud-dashboard.png)

> **⚠️ Importante:** Você precisa de um plano **Diamond** ou superior para usar VLAN na Discloud.

#### 2.3 Fazer upload do zip

Clique em **Add App** (ou no botão de upload) na dashboard da Discloud e envie o arquivo zip que você baixou no passo 2.1.

![Página de upload da dashboard da Discloud](images/deploy/discloud-upload.png)

#### 2.4 Encontrar o link de login do Tailscale

Após a aplicação iniciar, vá até a seção de **Logs** da sua aplicação e ative o **Auto-Reload**. Aguarde até que os logs mostrem uma URL de login do Tailscale. Ela será algo como:

```
To authenticate, visit: https://login.tailscale.com/a/XXXXXXXXXX
```

![Painel de logs da Discloud com auto-reload ativado mostrando a URL de autenticação do Tailscale](images/deploy/discloud-logs-tailscale-url.png)

#### 2.5 Autorizar o nó

Abra a URL de login do Tailscale dos logs no seu navegador. Faça login com a mesma conta do Tailscale que você criou anteriormente e **aprove a conexão**.

![Página de autorização do Tailscale pedindo para aprovar o novo nó](images/deploy/tailscale-authorize-node.png)

#### 2.6 Verificar a máquina no Tailscale

Volte ao console de administração do Tailscale em [https://login.tailscale.com/admin/machines](https://login.tailscale.com/admin/machines) e verifique se uma nova máquina chamada **`tailscale-discloud`** aparece na lista.

![Console de administração do Tailscale mostrando a máquina tailscale-discloud na lista de Machines](images/deploy/tailscale-machines-discloud.png)

#### 2.7 Ativar a VLAN na Discloud

Vá nas **Configurações** da aplicação Tailscale Tunnel Manager na dashboard da Discloud. Encontre a seção **VLAN**, **ative-a** e clique em **Salvar**.

Isso permite que o container do Tailscale se comunique com outras aplicações na mesma conta da Discloud pela rede interna.

![Página de configurações da aplicação na Discloud com o toggle de VLAN ativado](images/deploy/discloud-vlan-enable.png)

#### 2.8 Acessar o painel web

Na sua máquina local (que está conectada ao Tailscale), abra um navegador e acesse:

```
http://tailscale-discloud:3000
```

Você deve ver o painel web do Tailscale Tunnel Manager.

![Painel web do Tailscale Tunnel Manager carregado no navegador em http://tailscale-discloud:3000](images/banner.png)

---

### 3. Utilização — Criando seu primeiro túnel

Este exemplo mostra como configurar um túnel para uma instância MySQL hospedada na Discloud usando o template oficial do MySQL.

#### 3.1 Hospedar uma instância MySQL a partir de um template

Acesse a página do template MySQL da Discloud em [https://discloud.com/templates/1753305454851mysql](https://discloud.com/templates/1753305454851mysql), configure as opções como preferir e faça o deploy.

![Página do template MySQL da Discloud com opções de configuração](images/deploy/discloud-mysql-template.png)

#### 3.2 Configurar a VLAN na aplicação MySQL

Após o template do MySQL ser hospedado, vá nas **Configurações** dele na dashboard da Discloud e navegue até a seção **VLAN**.

Para o template do MySQL, a VLAN já está ativada e o hostname padrão é **`mysql`**.

![Configurações da aplicação MySQL na Discloud mostrando a VLAN ativada com hostname definido como mysql](images/deploy/discloud-mysql-vlan.png)

> **⚠️ Importante:** Para outras aplicações, você precisa ativar a VLAN manualmente e definir um hostname único. **Não use hostnames duplicados** entre suas aplicações — cada aplicação deve ter um hostname de VLAN distinto, caso contrário o roteamento interno não funcionará corretamente.

#### 3.3 Abrir o painel do Tunnel Manager

Na sua máquina local, abra o painel do Tunnel Manager no navegador:

```
http://tailscale-discloud:3000
```

#### 3.4 Criar um novo túnel

Clique em **New Tunnel** para criar um túnel. Você pode selecionar o template **MySQL** nos templates de início rápido — ele já vem preenchido com as configurações padrão para uma instância MySQL.

Aqui está o que cada parâmetro significa:

| Parâmetro | Descrição | Exemplo |
| --------- | --------- | ------- |
| **Name** | Um nome amigável para identificar o túnel. | `MySQL` |
| **Local Port** | A porta exposta no nó Tailscale. Esta é a porta que você usará para se conectar a partir da sua máquina local. Você pode alterá-la se o padrão conflitar com outra coisa. | `3306` |
| **Target Host** | O hostname VLAN da aplicação que você deseja alcançar. Deve corresponder ao hostname configurado nas configurações de VLAN da aplicação alvo. | `mysql` |
| **Target Port** | A porta em que a aplicação alvo está escutando. | `3306` |

![Formulário de novo túnel do Tunnel Manager com o template MySQL selecionado mostrando os campos de parâmetros](images/deploy/tunnel-manager-new-tunnel.png)

> **💡 Dica:** Você pode alterar a **Local Port** para qualquer porta disponível se a padrão já estiver em uso na instância do Tailscale. O **Target Host** e a **Target Port** devem corresponder ao hostname da VLAN e à porta de escuta da aplicação de destino.

#### 3.5 Verificar se o túnel está ativo

Após criar o túnel, verifique a seção **Active Tunnels** no painel. Seu túnel MySQL deve aparecer com o status **Online**.

![Painel do Tunnel Manager mostrando o túnel MySQL na lista de Active Tunnels com status Online](images/deploy/tunnel-manager-active-tunnels.png)

#### 3.6 Obter a string de conexão

Na tabela de **Active Tunnels**, veja a coluna **Connection**. Ela mostra o endereço que você deve usar para se conectar ao serviço a partir da sua máquina local.

Para o template padrão do MySQL, a string de conexão será:

```
tailscale-discloud:3306
```

Isso significa:

- **Host / Hostname / Domínio:** `tailscale-discloud`
- **Porta:** `3306`

Use esses valores em qualquer cliente MySQL, aplicação ou string de conexão.

#### 3.7 Conectar a partir da sua máquina local

Abra seu cliente de banco de dados preferido (este exemplo usa o **Tabularis**) e crie uma nova conexão usando os detalhes do túnel:

- **Host:** `tailscale-discloud`
- **Porta:** `3306`
- **Usuário / Senha:** As credenciais que você configurou ao hospedar o template do MySQL.

![Cliente de banco de dados Tabularis conectado à instância MySQL através do túnel Tailscale](images/deploy/tabularis-mysql-connection.png)

Pronto! Sua instância MySQL rodando na Discloud agora está acessível de forma segura a partir da sua máquina local através da rede Tailscale. Nenhuma porta é exposta na internet pública — todo o tráfego flui pela sua tailnet privada. 🎉
